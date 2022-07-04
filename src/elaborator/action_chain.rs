
use std::{intrinsics::transmute, mem::{size_of, forget, }, };

use crate::{
  support_structures::{mini_vector::SomeInlineVector,
    no_bullshit_closure::SomeSendableClosure}, };

use super::frame_allocator::{
  MemorySlabControlItem, SlabSize, GranularSlabAllocator, RCTaskBox};




#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum LinkKind {
  Step, Completion, FrameRequest,
  ProgressCheck, Gateway, TaskLocalClosure
}
#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum DataFrameSize {
  Absent, Bytes120, Bytes248, Bytes504, Bytes56
}


pub struct TaskHandle(
  pub(super) *mut dyn SomeInlineVector<Item = Task>,
  pub(super) MemorySlabControlItem,
  pub(super) *mut GranularSlabAllocator);

impl TaskHandle {
  pub fn assign_work_for_schedule(&self, item: ActionLink) {
    let task =
      Task::init(self.1, item);
    unsafe { (&mut *self.0).push(task) };
  }
  pub fn interpret_frame<T>(&self) -> &mut T {
    let size = self.1.project_size();
    let size = match size {
      SlabSize::Bytes64 => 56,
      SlabSize::Bytes128 => 120,
      SlabSize::Bytes256 => 248,
      SlabSize::Bytes512 => 504,
    };
    if size_of::<T>() > size {
      panic!("Attempt to interpret task frame as object that is bigger then frame itself");
    }
    return unsafe { &mut *self.1.project_slab_ptr().cast::<T>() }
  }
  pub fn get_parrent_frame(&self) -> Option<Self> {
    let size = self.1.project_size();
    let offset = match size {
      SlabSize::Bytes64 => 56usize,
      SlabSize::Bytes128 => 120,
      SlabSize::Bytes256 => 248,
      SlabSize::Bytes512 => 504,
    };
    unsafe {
      let ptr =
        self.1.project_slab_ptr().cast::<u8>().add(offset);
      let frame = *ptr.cast::<MemorySlabControlItem>();
      if frame.is_null() { return None }
      return Some(Self(self.0, frame, self.2));
    }
  }
  pub(super) fn request_slab(&self, slab_size: SlabSize)
  -> MemorySlabControlItem {
    unsafe { (&mut *self.2).acquire_memory(slab_size) }
  }
  pub fn spawn_box<T>(&self, value: T) -> RCTaskBox<T> { unsafe {
    let size = match size_of::<(MemorySlabControlItem, T)>() {
      0 ..= 65 => SlabSize::Bytes64,
      0 ..= 129 => SlabSize::Bytes128,
      0 ..= 257 => SlabSize::Bytes256,
      0 ..= 513 => SlabSize::Bytes512,
      _ => panic!("Value is too big to fit into task local box")
    };
    let mem = self.request_slab(size);
    let ptr =
      mem.project_slab_ptr().cast::<(u64, MemorySlabControlItem, T)>();
    ptr.write((0, mem, value));
    return RCTaskBox { storage_ptr: ptr }
  } }
  pub fn recycle_box<T>(&self, task_box: RCTaskBox<T>) { unsafe {
    let (rc, mci, val) =
      task_box.storage_ptr.read();
    if rc != 0 { panic!("Attempt to recycle a box which is referenced somewhere!") }
    drop(val);
    forget(task_box);
    (&mut *self.2).release_memory(mci)
  } }
}

#[derive(Clone, Copy, Debug)]
pub struct ActionLink(u64);
impl ActionLink {
  pub fn make_gateway(
    closure: SomeSendableClosure<TaskHandle, Self>
  ) -> Self {
    let boxed_clos = Box::new(closure);
    let mut gateway_ptr = unsafe { transmute::<_, u64>(boxed_clos) };
    gateway_ptr = (gateway_ptr << 4) + LinkKind::Gateway as u64;
    let link = Self(gateway_ptr);
    return link;
  }
  pub fn make_frame_request(
    frame_size: DataFrameSize,
    action_chain_head: ActionLink
  ) -> Self {
    let number =
      (((action_chain_head.0 << 4) + frame_size as u64) << 4)
      + LinkKind::FrameRequest as u64;
    return Self(number);
  }
  pub fn project_gateway(&self)
  -> SomeSendableClosure<TaskHandle, Self> { unsafe {
    let ptr = self.0 >> 4;
    let gw =
      transmute::<
        _,
        Box<SomeSendableClosure<TaskHandle, Self>>>
      (ptr);
    return *gw;
  } }
  pub fn project_link(&self) -> ActionLink {
    ActionLink(self.0 >> 8)
  }
  pub fn make_completion(should_delete_frame: bool) -> Self {
    return Self(((
      should_delete_frame as u64) << 4) + LinkKind::Completion as u64);
  }
  pub fn goto(fun_ptr: fn (TaskHandle) -> ActionLink) -> Self {
    unsafe {
      Self((transmute::<_, u64>(fun_ptr) << 4) + LinkKind::Step as u64) }
  }
  pub fn make_progress_checker(
    cheker_ptr: fn (TaskHandle) -> Option<ActionLink>
  ) -> Self {
    unsafe {
      Self((transmute::<_, u64>(cheker_ptr) << 4)
        + LinkKind::ProgressCheck as u64) }
  }
  pub fn make_task_local_closure<T>(
    handle: &TaskHandle, env: T, fun: fn (*mut T, TaskHandle) -> ActionLink
  ) -> ActionLink {
    //type CellContent = (MemorySlabControlItem, T, );
    let slab_size = match size_of::<(MemorySlabControlItem, *mut (), T)>() {
      0 ..= 65 => SlabSize::Bytes64,
      0 ..= 129 => SlabSize::Bytes128,
      0 ..= 257 => SlabSize::Bytes256,
      0 ..= 513 => SlabSize::Bytes512,
      _ => panic!("Too big size of environment")
    };
    let mem = handle.request_slab(slab_size);
    let ptr = mem.project_slab_ptr();
    let ptr_ = ptr.cast::<(MemorySlabControlItem, *mut (), T)>();
    unsafe { ptr_.write((mem, fun as *mut (), env)) };

    let num = ((ptr as u64) << 4) + LinkKind::TaskLocalClosure as u64;
    return ActionLink(num);
  }
  pub fn project_closure_ptr(&self) -> *mut () {
    (self.0 >> 4) as *mut ()
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
  pub fn project_fun_ptr (&self) -> fn (TaskHandle) -> ActionLink {
    return unsafe { transmute(self.0 >> 4) }
  }
  pub fn project_progress_checker(&self)
    -> fn (TaskHandle) -> Option<ActionLink> {
    return unsafe { transmute(self.0 >> 4) };
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Task {
  data_frame_ptr: MemorySlabControlItem,
  action_chain: ActionLink,
}
impl Task {
  pub fn init(
    data_frame: MemorySlabControlItem,
    action_chain_head: ActionLink,
  ) -> Self {
    return Self {
      action_chain: action_chain_head,
      data_frame_ptr: data_frame
    };
  }
  pub fn project_action_chain(&self) -> ActionLink {
    return self.action_chain;
  }
  pub fn inject_action_chain(&mut self, action: ActionLink) {
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
  pub fn project_func_ptr(&self) -> fn (TaskHandle) -> ActionLink {
    return self.action_chain.project_fun_ptr();
  }
  pub fn reached_completion(&self) -> bool {
    let kind = self.project_tag();
    return if let LinkKind::Completion = kind { true } else { false }
  }
}

