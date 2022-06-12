
use std::{
  thread::{JoinHandle, spawn, park, yield_now},
  sync::{
    Mutex, atomic::{AtomicBool, Ordering, AtomicU16}},
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
  liveness_count: AtomicU16,
}

enum RetirementChoise { Suspend, Terminate, Continue }


// tbd:
// • Should the impl utilise the defference
//   between `spawned subtasks` and `blocked tasks` ?
//   This can make loop more efficient but it has to be
//   decided on what amount of generated subtasks is too big to have
//   for oneself for too long.
//   Maybe, task cache size is enough to warrant a descision ?

fn elab_worker_task_loop
  <const TASK_CACHE_SIZE : usize>(
  stop_flag_ref: &AtomicBool,
  queue_ref: &mut LoopQueue<Task>,
  threads: *mut Vec<JoinHandle<()>>,
  liveness_count: &AtomicU16,
) {
  assert!(
    TASK_CACHE_SIZE <= u8::MAX as usize, "Too much of cache is bad for anyone!");
  let mut task_frame_allocator =
    GranularSlabAllocator::init_new();
  let mut task_cache: [MaybeUninit<Task> ; TASK_CACHE_SIZE] =
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
    let retire_strategy: RetirementChoise =
    queue_ref.with_acquired_queue(|queue| { unsafe {

      if queue.is_empty() {
        if !defered_tasks.is_empty() {
          // nothing on queue but something here; commit work, then reload.
          // this is unlikely scenario.
          // tbd: should pull in items before awaking?
          //  this may lower contention (?)
          for task in defered_tasks.drain(0 ..) {
            queue.enqueue_item(task);
          }
          // since there was nothing on queue, other threads might have chosen
          // to be suspended. unhibernate em now!
          for thread_handle in (&*threads).iter() {
            thread_handle.thread().unpark()
          }
        } else {
          // nothing was on queue and no pending tasks
          // were generated locally in previous quantum.
          // other threads migh still generate work.
          let lc =
            liveness_count.fetch_sub(1, Ordering::Relaxed) - 1;
          //println!("{}", lc);
          if lc == 0 { return RetirementChoise::Terminate; }
          return RetirementChoise::Suspend;
        }
      }
      // pull in some fresh items
      for i in 0 .. TASK_CACHE_SIZE as u16 {
        let item = queue.dequeue_item();
        match item {
          Some(item) => {
            task_cache.as_mut_ptr()
            .add(i as usize).cast::<Task>().write(item) ;
          },
          None => {
            limit = i;
            if !defered_tasks.is_empty() {
              // some work is there to commit from previous quantum
              for task in defered_tasks.drain(0 ..) {
                queue.enqueue_item(task);
              }
            }
            return RetirementChoise::Continue;
          },
        };
      }
      limit = TASK_CACHE_SIZE as u16;
      if !defered_tasks.is_empty() {
        // some work is there to commit from previous quantum
        for task in defered_tasks.drain(0 ..) {
          queue.enqueue_item(task);
        }
      }
      return RetirementChoise::Continue;
    } });
    match retire_strategy {
      RetirementChoise::Suspend => {
        // queue appear to be empty.
        // although it may be refilled later by other threads that
        // didnt commit their local work just yet.
        park();
        let _ = liveness_count.fetch_add(1, Ordering::Relaxed);
        //println!("{}", lc + 1);
        continue 'main;
      },
      RetirementChoise::Terminate => {
        // before you die, suggest to others to die as well!
        for thread_handle in unsafe {(&*threads).iter()} {
          thread_handle.thread().unpark()
        }
        break 'main
      },
      RetirementChoise::Continue => {},
    }

    // work on a couple of local tasks
    let mut index = 0u16;
    loop {
      let task = unsafe {
        (*task_cache.as_mut_ptr()
        .add(index as usize))
        .assume_init_mut()
      };
      'immidiate : loop {
        let action = task.project_action_chain();
        match action.project_tag() {
          LinkKind::FrameRequest => {
            // setup data frame for the task
            let link = action.project_link();
            task.inject_action_chain(link);
            // allocate task frame and set it for current task.
            // each task posesses a data frame that is shared with its subtasks.
            // todo: allow subtasks to request their own data frames
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
            // actually do something
            let work =
              action.project_fun_ptr();
            let df_ptr = task.project_data_frame_ptr();
            let tf_handle = TaskFrameHandle(df_ptr);
            let done_work = work(tf_handle);
            task.inject_action_chain(done_work);
            continue 'immidiate;
          },
          LinkKind::Fanout => {
            // current task want to spawn subtasks
            let df_ptr = task.project_data_frame_ptr();
            let tf_handle = TaskFrameHandle(df_ptr);
            let tg_handle =
              TaskGroupHandle(
                &mut defered_tasks, df_ptr);
            let setuper =
              action.project_setup_shim_ptr();
            let continuation = setuper(tf_handle, tg_handle);
            let mut dependent_task = *task;
            dependent_task.inject_action_chain(continuation);
            defered_tasks.push(dependent_task);
            // patch the hole !
            if limit == 1 {
              // nothing to patch. this quantum has complete.
              // sched subtasks & get new batch
              continue 'main;
            }
            if index == limit { // already at the end. just decrement end index
              limit -= 1;
            } else { // can pull item from end to current spot
              limit -= 1;
              unsafe {
                let patch =
                  task_cache
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
            // task is done
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
                  task_cache
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
            // some dependent task want to check in
            // to see if all of its blockers were resolved
            let checker =
              action.project_progress_checker();
            let df_ptr =
              TaskFrameHandle(task.project_data_frame_ptr());
            let smth = checker(df_ptr);
            if let Some(patch) = smth {
              // it can, indeed, continue
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
                    task_cache
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
  pub fn init(thread_count: u16, work_graph: ActionPtr) -> WorkGroupRef {
  unsafe {
    let mut wg =
      Box::<MaybeUninit<WorkGroup>>::new(MaybeUninit::uninit());
    let data = &mut *wg.as_mut_ptr() ;
    data.was_signaled_to_stop.store(false, Ordering::Relaxed);
    data.liveness_count.store(thread_count, Ordering::Relaxed);
    let q_ptr = addr_of_mut!(data.task_queue);
    let mut threads = Vec::<JoinHandle<()>>::new();
    threads.reserve(thread_count as usize);
    q_ptr.write(LoopQueue::init_new());
    let initial_task = Task::init(
      MemorySlabControlItem::init_null(),
      work_graph);
    wg.assume_init_mut().task_queue.with_acquired_queue(|queue|{
      queue.enqueue_item(initial_task);
    });
    // maybe it is reasonable to start threads with little relative
    // time difference rather then all at once?
    for _ in 0 .. thread_count {
      let queue_ref = &mut *q_ptr ;
      let stop_flag_ref = &data.was_signaled_to_stop;
      let threads_ptr = addr_of_mut!(data.threads) as usize;
      let lc = &data.liveness_count;
      let thread = spawn(move || {
        elab_worker_task_loop::<4>(
          stop_flag_ref, queue_ref,
          threads_ptr as *mut _, lc);
      });
      threads.push(thread);
    }
    addr_of_mut!(data.threads).write(threads);
    return WorkGroupRef(transmute(wg));
  } }
  pub fn await_completion(self) {
    yield_now(); // most likely a good descision
    for thread in self.0.threads {
      let _ = thread.join().unwrap();
    }
  }
  pub fn signal_to_stop(&self) {
    self.0.was_signaled_to_stop.fetch_or(true, Ordering::Relaxed);
  }
}