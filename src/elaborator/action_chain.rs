
use std::{intrinsics::transmute, mem::{size_of, forget, }, sync::atomic::AtomicU32, };

use crate::{
  support_structures::{mini_vector::SomeInlineVector,
    no_bullshit_closure::SomeSendableClosure}, };

use super::frame_allocator::{
  MemorySlabControlItem, SlabSize, GranularSlabAllocator, RCTaskBox};




#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum LinkKind {
  Step, Completion, FrameRequest,
  Gateway, TaskLocalClosure
}
#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum DataFrameSize {
  Absent, AproxBytes128, AproxBytes256, AproxBytes512, AproxBytes64
}


pub struct TaskHandle(
  pub(super) *mut dyn SomeInlineVector<Item = Task>,
  pub(super) MemorySlabControlItem,
  pub(super) *mut GranularSlabAllocator,
  pub(super) *mut u32);

impl TaskHandle {
  pub fn assign_work_for_schedule(&self, item: ActionLink) {
    let task =
      Task::init(self.1, item);
    unsafe {
      (&mut *self.0).push(task);
      *self.3 += 1;
    };
  }
  pub fn interpret_frame<T>(&self) -> &mut T {
    let mtd_size = size_of::<TaskMetadata>();
    let size = self.1.project_size();
    let size = match size {
      SlabSize::Bytes64 => 64,
      SlabSize::Bytes128 => 128,
      SlabSize::Bytes256 => 256,
      SlabSize::Bytes512 => 512,
    } - mtd_size;
    if size_of::<T>() > size {
      panic!("Attempt to interpret task frame as an object that is bigger then the frame itself");
    }
    return unsafe { &mut *self.1.project_slab_ptr().cast::<T>() }
  }
  pub fn get_parrent_frame(&self) -> Option<Self> {
    let mtd_size = size_of::<TaskMetadata>();
    let size = self.1.project_size();
    let offset = match size {
      SlabSize::Bytes64 => 64,
      SlabSize::Bytes128 => 128,
      SlabSize::Bytes256 => 256,
      SlabSize::Bytes512 => 512,
    } - mtd_size;
    unsafe {
      let ptr =
        self.1.project_slab_ptr().cast::<u8>().add(offset);
      let mtd = &*ptr.cast::<TaskMetadata>();
      let parrent_frame = mtd.parrent_frame_mtd;
      if parrent_frame.is_null() { return None }
      return Some(Self(self.0, parrent_frame, self.2, self.3));
    }
  }
  pub(super) fn request_slab(&self, slab_size: SlabSize)
  -> MemorySlabControlItem {
    unsafe { (&mut *self.2).acquire_memory(slab_size) }
  }
  pub fn spawn_box<T>(&self, value: T) -> RCTaskBox<T> { unsafe {
    let size = match size_of::<(T, MemorySlabControlItem, u64)>() {
      0 ..= 64 => SlabSize::Bytes64,
      0 ..= 128 => SlabSize::Bytes128,
      0 ..= 256 => SlabSize::Bytes256,
      0 ..= 512 => SlabSize::Bytes512,
      _ => panic!("Value is too big to fit into task local box")
    };
    let mem = self.request_slab(size);
    let ptr =
      mem.project_slab_ptr().cast::<(T, MemorySlabControlItem, u64)>();
    ptr.write((value, mem, 0));
    return RCTaskBox { storage_ptr: ptr }
  } }
  pub fn recycle_box<T>(&self, task_box: RCTaskBox<T>) { unsafe {
    let (val, mci, rc) =
      task_box.storage_ptr.read();
    if rc != 0 { panic!("Attempt to recycle a box which is referenced somewhere!") }
    drop(val);
    forget(task_box);
    (&mut *self.2).release_memory(mci)
  } }
}

const LINK_TAG_SIZE : usize = 4;
const LINK_TAG_MASK : usize = (1 << LINK_TAG_SIZE) - 1;
const MTD_SIZE : u64 = 5;
const IS_POLLER : u64 = 1 << LINK_TAG_SIZE;

#[derive(Clone, Copy, Debug)]
pub struct ActionLink(u64);
impl ActionLink {
  pub fn make_gateway(
    closure: SomeSendableClosure<TaskHandle, Self>
  ) -> Self {
    let boxed_clos = Box::new(closure);
    let mut gateway_ptr = unsafe { transmute::<_, u64>(boxed_clos) };
    gateway_ptr = (gateway_ptr << MTD_SIZE) + LinkKind::Gateway as u64;
    let link = Self(gateway_ptr);
    return link;
  }
  pub fn make_frame_request(
    frame_size: DataFrameSize,
    action_chain_head: ActionLink
  ) -> Self {
    let framed = (action_chain_head.0 << 4) + frame_size as u64;
    let tagged =
      (framed << MTD_SIZE) + LinkKind::FrameRequest as u64;
    return Self(tagged);
  }
  pub fn project_gateway(&self)
  -> SomeSendableClosure<TaskHandle, Self> { unsafe {
    let ptr = self.0 >> MTD_SIZE;
    let gw =
      transmute::<
        _,
        Box<SomeSendableClosure<TaskHandle, Self>>>
      (ptr);
    return *gw;
  } }
  pub fn project_link(&self) -> ActionLink {
    ActionLink(self.0 >> (MTD_SIZE + 4)) // this is correct
  }
  pub fn make_completion() -> Self {
    return Self(LinkKind::Completion as u64);
  }
  pub fn goto(fun_ptr: fn (TaskHandle) -> ActionLink) -> Self {
    unsafe {
      Self((transmute::<_, u64>(fun_ptr) << MTD_SIZE)
      + LinkKind::Step as u64) }
  }
  pub fn make_task_local_closure<T>(
    handle: &TaskHandle, env: T, fun: fn (*mut T, TaskHandle) -> ActionLink
  ) -> ActionLink {
    //type CellContent = (MemorySlabControlItem, T, );
    let slab_size = match size_of::<(MemorySlabControlItem, *mut (), T)>() {
      0 ..= 64 => SlabSize::Bytes64,
      0 ..= 128 => SlabSize::Bytes128,
      0 ..= 256 => SlabSize::Bytes256,
      0 ..= 512 => SlabSize::Bytes512,
      _ => panic!("Too big size of environment")
    };
    let mem = handle.request_slab(slab_size);
    let ptr = mem.project_slab_ptr();
    let ptr_ = ptr.cast::<(MemorySlabControlItem, *mut (), T)>();
    unsafe { ptr_.write((mem, fun as *mut (), env)) };

    let num = ((ptr as u64) << MTD_SIZE)
      + LinkKind::TaskLocalClosure as u64;
    return ActionLink(num);
  }
  pub fn project_closure_ptr(&self) -> *mut () {
    (self.0 >> MTD_SIZE) as *mut ()
  }
  pub fn project_tag (&self) -> LinkKind {
    unsafe { transmute((self.0 as u8) & (LINK_TAG_MASK as u8)) }
  }
  pub fn project_frame_size(&self) -> DataFrameSize {
    let untagged = (self.0 as u8) >> MTD_SIZE;
    let frame = untagged & ((1 << 4) - 1);
    return unsafe { transmute(frame) }
  }
  pub fn project_fun_ptr (&self) -> fn (TaskHandle) -> ActionLink {
    return unsafe { transmute(self.0 >> MTD_SIZE) }
  }
  pub fn project_progress_checker(&self)
    -> fn (TaskHandle) -> Option<ActionLink> {
    return unsafe { transmute(self.0 >> MTD_SIZE) };
  }
  pub fn mark_as_poller(&mut self) {
    self.0 = self.0 | IS_POLLER
  }
  pub fn is_poller(&self) -> bool {
    (self.0 & IS_POLLER) != 0
  }
  pub fn unmark_as_poller(&mut self) {
    self.0 = self.0 & !IS_POLLER
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
  pub fn mark_as_poller(&mut self) {
    self.action_chain.mark_as_poller()
  }
  pub fn unmark_as_poller(&mut self) {
    self.action_chain.unmark_as_poller()
  }
}

pub struct TaskMetadata {
  pub await_counter: AtomicU32,
  pub parrent_frame_mtd: MemorySlabControlItem
}