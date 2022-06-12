
use std::{
  thread::{JoinHandle, spawn},
  sync::{
    Mutex, atomic::{AtomicBool, Ordering}},
    mem::{MaybeUninit, size_of},
    ptr::addr_of_mut, intrinsics::transmute,
    alloc::{Layout, alloc}};
use crate::elaborator::{
  frame_allocator::{GranularSlabAllocator, SlabSize}, action_chain::{DataFrameSize, TaskFrameHandle}};

use super::{action_chain::{Task, LinkKind, ActionPtr, TaskGroupHandle}, frame_allocator::MemorySlabControlItem};

#[derive(Debug)]
pub struct LoopData<T> {
  pub read_page: *mut T,
  pub write_page: *mut T,
  newest_page: *mut T,
  write_ptr: u16,
  read_ptr: u16,
  //#[cfg(test)]
  //pub number_of_allocated_pages: usize,
}
impl<T> LoopData<T> {
  pub fn is_empty(&self) -> bool {
    self.read_page == self.write_page &&
    self.read_ptr == self.write_ptr
  }
  pub fn enqueue_item (&mut self, item: T) { unsafe {
    self.write_page.add(self.write_ptr as usize).write(item) ;
    self.write_ptr += 1;
    if self.write_ptr as usize == 4096 / size_of::<T>() {
      let new_initial_position = (8 / size_of::<T>()).max(1) as u16;
      self.write_ptr = new_initial_position;
      let ptr = *self.write_page.cast::<usize>();
      if ptr == usize::MAX { // no more available space
        let fresh_page =
          alloc(Layout::from_size_align_unchecked(4096, 1));
        // if cfg!(test) { self.total_allocated_pages += 1 };
        // { self.number_of_allocated_pages += 1 };
        *fresh_page.cast::<usize>() = usize::MAX;
        *self.newest_page.cast::<*mut u8>() = fresh_page;
        self.newest_page = fresh_page.cast();
        self.write_page = fresh_page.cast();
      } else { // just switch write page
        self.write_page = ptr as *mut T;
      }
    }
  } }
  pub fn dequeue_item (&mut self) -> Option<T> { unsafe {
    if self.is_empty() { return None; };
    let item = self.read_page.add(self.read_ptr as usize).read();
    self.read_ptr += 1;
    if self.read_ptr as usize == 4096 / size_of::<T>() {
      let ptr_ref = self.read_page.cast::<usize>();
      self.read_ptr = (8 / size_of::<T>()).max(1) as u16;
      if *ptr_ref != usize::MAX {
        let new_read_page = *ptr_ref as *mut T;
        *ptr_ref = usize::MAX;
        *self.newest_page.cast() = self.read_page;
        self.newest_page = self.read_page;
        self.read_page = new_read_page;
      } else {
        // rare case when # of reads == # of writes and page is filled exactly
        // to the limit
        self.write_ptr = self.read_ptr;
      }
    };
    return Some(item);
  } }
  pub fn init_new () -> Self { unsafe {
    let page =
      alloc(Layout::from_size_align_unchecked(4096, 1))
      .cast::<T>() ;
    *page.cast::<usize>() = usize::MAX;
    let initial_position = (8 / size_of::<T>()).max(1) as u16;
    return Self {
      read_page: page,
      write_page: page,
      newest_page: page,
      read_ptr: initial_position,
      write_ptr: initial_position,
      //number_of_allocated_pages: 1,
    }
  } }
}

// mutex is used because rust doesnt have 16 byte atomics
// in stable, but this is fine, cus I either way not sure
// if this amount of state can be synced atomically
pub struct LoopQueue<T>(Mutex<LoopData<T>>);
impl <T> LoopQueue<T> {
  pub fn init_new () -> Self {
    Self(Mutex::new(LoopData::init_new()))
  }
  pub fn with_acquired_queue<K>(
    &mut self,
    action: impl FnOnce(&mut LoopData<T>) -> K
  ) -> K {
    let mut mutex = self.0.lock().unwrap();
    let smth = action(&mut mutex);
    drop(mutex);
    return smth;
  }
}
unsafe impl <T> Send for LoopQueue<T> {}

pub struct WorkGroup {
  threads: Vec<JoinHandle<()>>,
  task_queue: LoopQueue<Task>,
  was_signaled_to_stop: AtomicBool,
}


fn elab_worker_task_loop
  <const CacheSize : usize>(
  stop_flag_ref: &AtomicBool,
  queue_ref: &mut LoopQueue<Task>
) {
  assert!(
    CacheSize <= u8::MAX as usize, "Too much of cache is bad for anyone!");
  let mut task_frame_allocator =
    GranularSlabAllocator::init_new();
  let mut local_tasks: [MaybeUninit<Task> ; CacheSize] =
    unsafe { MaybeUninit::uninit().assume_init() };
  let mut limit: u16 = 0;
  let mut defered_tasks = Vec::<Task>::new();
  defered_tasks.reserve(32);
  'main : loop {
    if stop_flag_ref.load(Ordering::Relaxed) { break 'main; };
    // take a couple of tasks to this thread's storage
    // to not put much pressure on mutex by frequent retriewing
    // of items from work queue. It might appear
    // because majority of tasks are expected to be quiete cheap to execute
    let queue_is_empty =
    queue_ref.with_acquired_queue(|queue| {
      // fixme: pushing to queue has to be done after
      // retreiving !!
      if !defered_tasks.is_empty() {
        for task in defered_tasks.drain(0 ..) {
          queue.enqueue_item(task);
        }
      }
      if queue.is_empty() { return true; }
      for i in 0 .. CacheSize as u16 {
        let item = queue.dequeue_item();
        match item {
          Some(item) => {
            unsafe {
              local_tasks.as_mut_ptr()
              .add(i as usize).cast::<Task>().write(item) };
          },
          None => {
            limit = i; return false;
          },
        };
      }
      limit = CacheSize as u16;
      return false;
    });
    if queue_is_empty {
      // this must mean that all assigned work has been complete.
      // nothing more to do here
      break 'main;
    }
    // work on a couple of local tasks
    let mut index = 0u16;
    loop {
      let task = unsafe {
        (*local_tasks.as_mut_ptr()
        .add(index as usize))
        .assume_init_mut()
      };
      'immidiate : loop {
        let action = task.project_action_chain();
        match action.project_tag() {
          LinkKind::FrameRequest => {
            let link = action.project_link();
            task.inject_action_chain(link);
            // allocate task frame and set it for current task
            let frame_request = action.project_frame_size();
            let mem = match frame_request {
              DataFrameSize::Absent => {
                MemorySlabControlItem::init_null()
              },
              DataFrameSize::Bytes128 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes128)
              },
              DataFrameSize::Bytes256 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes256)
              },
              DataFrameSize::Bytes512 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes512)
              },
            };
            task.inject_data_frame_ptr(mem);
            continue 'immidiate;
          },
          LinkKind::Step => {
            let work =
              action.project_fun_ptr();
            let df_ptr = task.project_data_frame_ptr();
            let tf_handle = TaskFrameHandle(df_ptr);
            let done_work = work(tf_handle);
            task.inject_action_chain(done_work);
            continue 'immidiate;
          },
          LinkKind::Fanout => {
            let df_ptr = task.project_data_frame_ptr();
            let tf_handle = TaskFrameHandle(df_ptr);
            let handle =
              TaskGroupHandle(
                &mut defered_tasks, df_ptr);
            let setuper =
              action.project_setup_shim_ptr();
            let continuation = setuper(tf_handle, handle);
            let dependent_task =
              Task::init(
                df_ptr, continuation);
            defered_tasks.push(dependent_task);
            // patch the hole !
            if limit == 1 { // nothing to patch. sched subtasks & get new batch
              continue 'main;
            }
            if index == limit { // already at the end. just decrement end index
              limit -= 1;
            } else { // can pull item from end to current spot
              limit -= 1;
              unsafe {
                let patch =
                  local_tasks
                  .as_ptr()
                  .add(limit as usize)
                  .read()
                  .assume_init();
                *task = patch;
              };
              continue 'immidiate;
            }
          },
          LinkKind::Completion => {
            let should_release = action.project_deletion_flag();
            if should_release {
              task_frame_allocator.release_memory(
                task.project_data_frame_ptr())
            }
            if limit == 1 { // nothing to patch. sched subtasks & get new batch
              continue 'main;
            }
            if index == limit { // already at the end. just decrement end index
              limit -= 1;
            } else { // can pull item from end to current spot
              limit -= 1;
              unsafe {
                let patch =
                  local_tasks
                  .as_ptr()
                  .add(limit as usize)
                  .read()
                  .assume_init();
                *task = patch;
              };
              continue 'immidiate;
            }
          },
          LinkKind::ProgressCheck => {
            let checker =
              action.project_progress_checker();
            let df_ptr =
              TaskFrameHandle(task.project_data_frame_ptr());
            let smth = checker(df_ptr);
            if let Some(patch) = smth {
              task.inject_action_chain(patch);
              continue 'immidiate;
            } else {
              defered_tasks.push(*task);
              if limit == 1 { // nothing to patch. sched subtasks & get new batch
                continue 'main;
              }
              if index == limit { // already at the end. just decrement end index
                limit -= 1;
              } else { // can pull item from end to current spot
                limit -= 1;
                unsafe {
                  let patch =
                    local_tasks
                    .as_ptr()
                    .add(limit as usize)
                    .read()
                    .assume_init();
                  *task = patch;
                };
                continue 'immidiate;
              }
            }
          },
          LinkKind::Callback => todo!(),
        }
        break 'immidiate;
      }
      if limit == 0 { continue 'main; }
      index += 1;
      if index == limit { index = 0 }
    };
  };
}

pub struct WorkGroupRef(Box<WorkGroup>);
impl WorkGroupRef {
  pub fn init(thread_count: usize, work_graph: ActionPtr) -> WorkGroupRef {
  unsafe {
    let mut wg =
      Box::<MaybeUninit<WorkGroup>>::new(MaybeUninit::uninit());
    let data = &mut *wg.as_mut_ptr() ;
    data.was_signaled_to_stop.store(false, Ordering::Relaxed);
    let q_ptr = addr_of_mut!(data.task_queue);
    let mut threads = Vec::<JoinHandle<()>>::new();
    threads.reserve(thread_count);
    q_ptr.write(LoopQueue::init_new());
    let initial_task = Task::init(
      MemorySlabControlItem::init_null(),
      work_graph);
    wg.assume_init_mut().task_queue.with_acquired_queue(|queue|{
      queue.enqueue_item(initial_task);
    });
    for _ in 0 .. thread_count {
      let queue_ref = &mut *q_ptr ;
      let stop_flag_ref = &data.was_signaled_to_stop;
      let thread = spawn(move || {
        elab_worker_task_loop::<4>(stop_flag_ref, queue_ref);
      });
      threads.push(thread);
    }
    addr_of_mut!(data.threads).write(threads);
    return WorkGroupRef(transmute(wg));
  } }
  pub fn await_completion(self) {
    for thread in self.0.threads {
      let _ = thread.join().unwrap();
    }
  }
}