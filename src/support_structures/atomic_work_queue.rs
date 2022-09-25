use std::{sync::atomic::{AtomicU64, Ordering, AtomicU32}, alloc::{alloc, Layout, alloc_zeroed}, mem::{align_of, size_of}, ptr::addr_of, arch::x86_64};

use crate::elaborator::action_chain::Task;


const QUEUE_BACKING_MEM_SIZE : usize = 1 << 13;
const TASK_QUEUE_CAPACITY : usize = (1 << 13) / size_of::<Task>();

#[derive(Debug)]
pub struct BoundedAtomicTaskQueue {
  mem_ptr: *mut Task,
  ptr_data: RWData,
}
pub struct SyncedPtrs<'i> {
  pub read_ptr: &'i AtomicU32,
  pub write_ptr: &'i AtomicU32,
}
#[derive(Debug)]
pub struct RWData(AtomicU64);
impl RWData {
  pub fn init() -> Self {
    let this = Self(AtomicU64::new(0));
    let SyncedPtrs { read_ptr, .. } = this.project_rw_ptr_data();
    read_ptr.store(TASK_QUEUE_CAPACITY as u32, Ordering::Relaxed);
    return this;
  }
  pub fn project_rw_ptr_data(&self) -> SyncedPtrs { unsafe {
    let ptr = addr_of!(self.0).cast::<AtomicU32>();
    let read_ptr = &*ptr;
    let write_ptr = &*ptr.add(1);
    return SyncedPtrs { read_ptr, write_ptr };
  } }
  pub fn get_synced_obj(&self) -> &AtomicU64 {
    &self.0
  }
}

impl BoundedAtomicTaskQueue {
  pub fn init() -> Self { unsafe {
    let raw_mem_ptr = alloc(
      Layout::from_size_align_unchecked(
        QUEUE_BACKING_MEM_SIZE, align_of::<Task>()));
    return Self {
      mem_ptr: raw_mem_ptr.cast(),
      ptr_data: RWData::init(),
    };
  } }
}

impl BoundedAtomicTaskQueue {
  pub fn try_enqueue_task(&self, task: Task) -> bool { unsafe {

    let SyncedPtrs { write_ptr , read_ptr } = self.ptr_data.project_rw_ptr_data();
    let mut fresh_slot_ptr = write_ptr.load(Ordering::Relaxed);
    loop {
      if fresh_slot_ptr == read_ptr.load(Ordering::Relaxed) {
        return false;
      }
      fresh_slot_ptr = write_ptr.fetch_add(1, Ordering::Relaxed);
      if fresh_slot_ptr as usize >= TASK_QUEUE_CAPACITY {
        let _ = write_ptr.compare_exchange(fresh_slot_ptr + 1, 0, Ordering::Relaxed, Ordering::Relaxed);
        continue;
      }
      break;
    }
    let slot_ptr = self.mem_ptr.add(fresh_slot_ptr as usize);
    *slot_ptr = task;
    return true;
  } }
  pub fn dequeue_task(&self) -> Option<Task> { unsafe {

    let SyncedPtrs { read_ptr, write_ptr } = self.ptr_data.project_rw_ptr_data();
    let mut actual = read_ptr.load(Ordering::Relaxed);
    loop {
      if actual == write_ptr.load(Ordering::Relaxed) {
        return None;
      };
      actual = read_ptr.fetch_add(1, Ordering::Relaxed);
      if actual as usize >= TASK_QUEUE_CAPACITY {
        let _ = read_ptr.compare_exchange(actual + 1, 0, Ordering::Relaxed, Ordering::Relaxed);
        continue;
      }
      break;
    }
    let slot_ptr = self.mem_ptr.add(actual as usize);

    return Some(*slot_ptr);
  } }
}