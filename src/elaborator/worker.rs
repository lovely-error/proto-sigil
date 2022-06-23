
use std::{
  thread::{JoinHandle, spawn, park, yield_now},
  sync::{
    Mutex, atomic::{AtomicBool, Ordering, AtomicU16, fence}},
    mem::{MaybeUninit, size_of},
    ptr::addr_of_mut, intrinsics::{transmute},
    alloc::{Layout, alloc}};
use crate::{elaborator::{
  frame_allocator::{GranularSlabAllocator, SlabSize,},
  action_chain::{DataFrameSize, TaskHandle,}},
  support_structures::mini_vector::InlineVector};

use super::{
  action_chain::{Task, LinkKind, ActionLink, TaskGroupHandle},
  frame_allocator::MemorySlabControlItem};

#[derive(Debug)]
pub struct LoopQueue<T> {
  pub read_page: *mut T,
  pub write_page: *mut T,
  pub newest_page: *mut T,
  pub write_ptr: u16,
  read_ptr: u16,
  //#[cfg(test)]
  //pub number_of_allocated_pages: usize,
}
impl<T> LoopQueue<T> {
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
pub struct WorkQueue<T>(Mutex<LoopQueue<T>>);
impl <T> WorkQueue<T> {
  pub fn init_new () -> Self {
    Self(Mutex::new(LoopQueue::init_new()))
  }
  pub fn with_acquired_queue<K>(
    &mut self,
    action: impl FnOnce(&mut LoopQueue<T>) -> K
  ) -> K {
    let mut mutex = self.0.lock().unwrap();
    let smth = action(&mut mutex);
    drop(mutex);
    return smth;
  }
}
unsafe impl <T> Send for WorkQueue<T> {}

pub struct WorkGroup {
  executors: Vec<JoinHandle<()>>,
  task_queue: WorkQueue<Task>,
  was_signaled_to_stop: AtomicBool,
  liveness_count: AtomicU16,
}


enum RetirementChoise { Suspend, Terminate, Continue }

fn elab_worker_task_loop
  <const TASK_CACHE_SIZE : usize>(
  stop_flag_ref: &AtomicBool,
  queue_ref: &mut WorkQueue<Task>,
  threads: *mut Vec<JoinHandle<()>>,
  liveness_count: &AtomicU16,
) {
  assert!(
    TASK_CACHE_SIZE <= u8::MAX as usize,
    "Too much of cache is bad for anyone!");
  let mut task_frame_allocator =
    GranularSlabAllocator::init_new();
  let mut task_cache: [MaybeUninit<Task> ; TASK_CACHE_SIZE] =
    unsafe { MaybeUninit::uninit().assume_init() };
  let mut limit: u16 = 0;
  let mut spawned_subtasks =
    InlineVector::<24, Task>::init();
  let mut pending_tasks =
    InlineVector::<6, Task>::init();
  'main : loop {
    if stop_flag_ref.load(Ordering::Relaxed) { break 'main; };
    // take a couple of tasks to this thread's storage
    // to not put much pressure on mutex by frequent retrieving
    // of items from work queue. It might appear
    // because majority of tasks are expected to be quiete cheap to execute
    let mut should_ping_threads : bool = false;
    let retire_strategy: RetirementChoise =
    queue_ref.with_acquired_queue(|queue| { unsafe {

      let did_produce_work =
          !spawned_subtasks.is_empty() || !pending_tasks.is_empty();
      if queue.is_empty() {
        if did_produce_work { should_ping_threads = true; }

        pending_tasks.copy_quickly_into(queue);
        pending_tasks.reset();
        spawned_subtasks.copy_quickly_into(queue);
        spawned_subtasks.reset();
        if !did_produce_work {
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
      let some_threads_are_dormant =
        liveness_count.load(Ordering::Relaxed) as usize
        != (&*threads).len();
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
            if did_produce_work && some_threads_are_dormant {
              should_ping_threads = true
            }
            spawned_subtasks.copy_quickly_into(queue);
            spawned_subtasks.reset();
            pending_tasks.copy_quickly_into(queue);
            pending_tasks.reset();
            return RetirementChoise::Continue;
          },
        };
      }
      limit = TASK_CACHE_SIZE as u16;
      if did_produce_work && some_threads_are_dormant {
        should_ping_threads = true
      }
      spawned_subtasks.copy_quickly_into(queue);
      spawned_subtasks.reset();
      pending_tasks.copy_quickly_into(queue);
      pending_tasks.reset();
      return RetirementChoise::Continue;
    } });
    if should_ping_threads {
      for thread_handle in unsafe { (&*threads).iter() } {
        thread_handle.thread().unpark()
      }
    }
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
    'that : loop {
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
            let frame_request = action.project_frame_size();
            let mem = match frame_request {
              DataFrameSize::Absent => {
                MemorySlabControlItem::init_null()
              },
              DataFrameSize::Bytes120 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes128)
              },
              DataFrameSize::Bytes248 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes256)
              },
              DataFrameSize::Bytes504 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes512)
              },
              DataFrameSize::Bytes56 => {
                task_frame_allocator.acquire_memory(SlabSize::Bytes64)
              },
            };
            // put a ptr to a parrent frame into any child that
            // wants its own memory.
            // root of the task tree point to null
            let parent_frame =
              task.project_data_frame_ptr();
            mem.inject_parent_frame(parent_frame);
            task.inject_data_frame_ptr(mem);
            continue 'immidiate;
          },
          LinkKind::Step => {
            // actually do something
            let work =
              action.project_fun_ptr();
            let df_ptr = task.project_data_frame_ptr();
            let tf_handle =
              TaskHandle(addr_of_mut!(spawned_subtasks), df_ptr);
            let done_work = work(tf_handle);
            task.inject_action_chain(done_work);
            continue 'immidiate;
          },
          LinkKind::Completion => {
            // task is done
            let should_release = action.project_deletion_flag();
            if should_release {
              task_frame_allocator.release_memory(
                task.project_data_frame_ptr());
            }
          },
          LinkKind::ProgressCheck => {
            // some dependent task want to check in
            // to see if all of its blockers were resolved
            let checker =
              action.project_progress_checker();
            let frame_ptr = TaskHandle(
              addr_of_mut!(spawned_subtasks), task.project_data_frame_ptr());
            let smth = checker(frame_ptr);
            if let Some(patch) = smth {
              // it can, indeed, continue
              task.inject_action_chain(patch);
              continue 'immidiate;
            } else {
              // put it in the wait corner
              pending_tasks.append(*task);
            }
          },
          LinkKind::Gateway => {
            let gw =
              action.project_gateway();
            let frame_handle = TaskHandle(
              addr_of_mut!(spawned_subtasks), task.project_data_frame_ptr());
            let next = gw.invoke_consume(frame_handle);
            task.inject_action_chain(next);
            continue 'immidiate;
          },
        }
        break 'immidiate;
      }
      index += 1;
      if index == limit {
        let len = spawned_subtasks.count_items() as usize;
        if len > 0 && len <= TASK_CACHE_SIZE {
          // can refill cache without going through queue
          limit = len as u16; index = 0;
          for i in 0 .. len {
            let item = spawned_subtasks.pop().unwrap();
            unsafe {
              task_cache.as_mut_ptr().add(i)
              .cast::<Task>().write(item);
            }
          }
          continue 'that;
        } else {
          continue 'main;
        }
      }
    };
  };

}

pub struct WorkGroupRef(Box<WorkGroup>);
impl WorkGroupRef {
  pub fn init(thread_count: u16, work_graph: ActionLink) -> WorkGroupRef {
  unsafe {
    let mut wg =
      Box::<MaybeUninit<WorkGroup>>::new(MaybeUninit::uninit());
    let data = &mut *wg.as_mut_ptr() ;
    data.was_signaled_to_stop.store(false, Ordering::Relaxed);
    data.liveness_count.store(thread_count, Ordering::Relaxed);
    let q_ptr = addr_of_mut!(data.task_queue);
    let mut threads = Vec::<JoinHandle<()>>::new();
    threads.reserve(thread_count as usize);
    q_ptr.write(WorkQueue::init_new());
    let initial_task = Task::init(
      MemorySlabControlItem::init_null(),
      work_graph);
    wg.assume_init_mut().task_queue.with_acquired_queue(|queue|{
      queue.enqueue_item(initial_task);
    });
    // maybe it is reasonable to start threads with little relative
    // time difference rather then all at once?
    fence(Ordering::Release);
    for _ in 0 .. thread_count {
      let queue_ref = &mut *q_ptr ;
      let stop_flag_ref = &data.was_signaled_to_stop;
      let threads_ptr = addr_of_mut!(data.executors) as usize;
      let lc = &data.liveness_count;
      let thread = spawn(move || {
        elab_worker_task_loop::<8>(
          stop_flag_ref, queue_ref,
          threads_ptr as *mut _, lc);
      });
      threads.push(thread);
    }
    addr_of_mut!(data.executors).write(threads);
    return WorkGroupRef(transmute(wg));
  } }
  pub fn await_completion(self) {
    yield_now(); // most likely a good descision
    for thread in self.0.executors {
      let _ = thread.join().unwrap();
    }
  }
  pub fn signal_to_stop(&self) {
    self.0.was_signaled_to_stop.store(true, Ordering::Relaxed);
  }
}

