
use std::{intrinsics::transmute, mem::{size_of, forget}, ptr::addr_of};

use crate::support_structures::mini_vector::SomeInlineVector;

use super::frame_allocator::{MemorySlabControlItem, SlabSize};

// Task interface
// enum ActionChain {

//   Gateway(Box<dyn FnOnce() -> Self>)

//   // a single step of computation
//   Step(fn (TaskFrameHandle) -> Self),

//   // used to spawn subtasks
//   Fanout(fn (TaskFrameHandle, TaskGroupHandle) -> Self),

//   // this will get scheduled nondeterministically
//   // to check if all dependencies were resolved
//   ProgressCheck(fn (TaskFrameHandle) -> Self),

//   // marks the endpoint of task lifetime.
//   // if task didnt inherit its frame from parrent
//   // and instead requested fresh one, then
//   // here it signals to release the resource
//   Completion { should_delete_frame: bool },

// }

#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum LinkKind {
  Step, Fanout, Completion, Callback, FrameRequest,
  ProgressCheck, Gateway
}
#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum DataFrameSize {
  Absent, Bytes120, Bytes248, Bytes504, Bytes56
}

pub struct TaskGroupHandle<'i>(
  pub(super) &'i mut dyn SomeInlineVector<Item = Task>, pub(super) MemorySlabControlItem);
impl TaskGroupHandle<'_> {
  pub fn assign_work(&mut self, item: ActionPtr) {
    let task =
      Task::init(self.1, item);
    self.0.push(task);
  }
}

pub struct TaskFrameHandle(pub MemorySlabControlItem);
impl TaskFrameHandle {
  pub fn interpret_frame<T>(&self) -> &mut T {
    let size = self.0.project_size();
    let size = match size {
      SlabSize::Bytes64 => 56,
      SlabSize::Bytes128 => 120,
      SlabSize::Bytes256 => 248,
      SlabSize::Bytes512 => 504,
    };
    if size_of::<T>() > size {
      panic!("Attempt to interpret task frame as object that is bigger then frame itself");
    }
    return unsafe { &mut *self.0.project_slab_ptr().cast::<T>() }
  }
  pub fn get_parrent_frame(&self) -> Option<Self> {
    let size = self.0.project_size();
    let offset = match size {
      SlabSize::Bytes64 => 56usize,
      SlabSize::Bytes128 => 120,
      SlabSize::Bytes256 => 248,
      SlabSize::Bytes512 => 504,
    };
    unsafe {
      let ptr =
        self.0.project_slab_ptr().cast::<u8>().add(offset);
      let frame = *ptr.cast::<MemorySlabControlItem>();
      if frame.is_null() { return None }
      return Some(Self(frame));
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct ActionPtr(u64);
impl ActionPtr {
  pub fn make_gateway(
    closure: Box<dyn FnOnce(TaskFrameHandle) -> Self + Send>
  ) -> Self { unsafe {
    let mut gateway_ptr = addr_of!(closure) as u64;
    gateway_ptr = (gateway_ptr << 4) + LinkKind::Gateway as u64;
    forget(closure);
    let link = transmute::<_, ActionPtr>(gateway_ptr);
    return link;
  } }
  pub fn make_frame_request(
    frame_size: DataFrameSize,
    action_chain_head: ActionPtr
  ) -> Self {
    let number =
      (((action_chain_head.0 << 4) + frame_size as u64) << 4)
      + LinkKind::FrameRequest as u64;
    return Self(number);
  }
  pub fn project_gateway(&self)
  -> Box<dyn FnOnce(TaskFrameHandle) -> Self + Send> { unsafe {
    let ptr = self.0 >> 4;
    let gw =
      transmute::<
        _,
        *const Box<dyn FnOnce(TaskFrameHandle) -> Self + Send>>
      (ptr);
    return gw.read();
  } }
  pub fn project_link(&self) -> ActionPtr {
    ActionPtr(self.0 >> 8)
  }
  pub fn make_completion(should_delete_frame: bool) -> Self {
    return Self(((
      should_delete_frame as u64) << 4) + LinkKind::Completion as u64);
  }
  pub fn make_link(
    kind: LinkKind,
    fun_ptr: fn (TaskFrameHandle) -> ActionPtr
  ) -> Self {
    unsafe { Self((transmute::<_, u64>(fun_ptr) << 4) + kind as u64) }
  }
  pub fn make_fanout(
    setuper_ptr: fn (TaskFrameHandle, TaskGroupHandle) -> ActionPtr
  ) -> Self {
    unsafe {
      Self((transmute::<_, u64>(setuper_ptr) << 4) + LinkKind::Fanout as u64) }
  }
  pub fn make_progress_checker(
    cheker_ptr: fn (TaskFrameHandle) -> Option<ActionPtr>
  ) -> Self {
    unsafe {
      Self((transmute::<_, u64>(cheker_ptr) << 4)
        + LinkKind::ProgressCheck as u64) }
  }
  pub fn project_deletion_flag(&self) -> bool {
    (self.0 >> 4) == 1
  }
  pub fn project_tag (&self) -> LinkKind {
    unsafe { transmute((self.0 as u8) & (1 << 4) - 1) }
  }
  pub fn project_frame_size(&self) -> DataFrameSize {
    unsafe { transmute(((self.0 as u8) >> 4) & (1 << 4) - 1) }
  }
  pub fn project_fun_ptr (&self) -> fn (TaskFrameHandle) -> ActionPtr {
    return unsafe { transmute(self.0 >> 4) }
  }
  pub fn project_setup_shim_ptr(&self)
    -> fn (TaskFrameHandle, TaskGroupHandle) -> ActionPtr {
    return unsafe { transmute(self.0 >> 4) };
  }
  pub fn project_progress_checker(&self)
    -> fn (TaskFrameHandle) -> Option<ActionPtr> {
    return unsafe { transmute(self.0 >> 4) };
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Task {
  data_frame_ptr: MemorySlabControlItem,
  action_chain: ActionPtr,
}
impl Task {
  pub fn init(
    data_frame: MemorySlabControlItem,
    action_chain_head: ActionPtr,
  ) -> Self {
    return Self {
      action_chain: action_chain_head,
      data_frame_ptr: data_frame
    };
  }
  pub fn project_action_chain(&self) -> ActionPtr {
    return self.action_chain;
  }
  pub fn inject_action_chain(&mut self, action: ActionPtr) {
    self.action_chain = action;
  }
  pub fn project_tag(&self) -> LinkKind {
    return self.action_chain.project_tag();
  }
  pub fn inject_data_frame_ptr(&mut self, dfp: MemorySlabControlItem) {
    self.data_frame_ptr = dfp;
  }
  pub fn project_data_frame_ptr(&self) -> MemorySlabControlItem {
    self.data_frame_ptr
  }
  pub fn project_func_ptr(&self) -> fn (TaskFrameHandle) -> ActionPtr {
    return self.action_chain.project_fun_ptr();
  }
  pub fn reached_completion(&self) -> bool {
    let kind = self.project_tag();
    return if let LinkKind::Completion = kind { true } else { false }
  }
}