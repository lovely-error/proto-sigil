

use std::{
  alloc::dealloc, mem::{size_of, align_of}, time::{SystemTime},
  intrinsics::{transmute}, ops::Shl, sync::{atomic::{AtomicU64, Ordering, AtomicBool}, Mutex}, ptr::addr_of_mut};

use proto_sigil::elaborator::{
  action_chain::{
    ActionPtr, LinkKind, DataFrameSize, TaskGroupHandle, TaskFrameHandle },
  worker::{WorkGroupRef, WorkGroup}, frame_allocator::PageHeaderData};

struct Example(bool);
impl Drop for Example {
  fn drop(&mut self) {
    println!("Eh, you dropped garbage, my friend...")
  }
}

//#[test]
fn drop_on_ptrs () {
  use std::alloc::{alloc, Layout};

  unsafe {
    let lay = Layout::new::<Example>();
    let mem_ptr = alloc(lay);
    //*mem_ptr.cast::<Example>() = Example(true);
    mem_ptr.cast::<Example>().write(Example(true));
    dealloc(mem_ptr, lay);
  };
}


//#[test]
fn size_test () {
  println!("{}", size_of::<Box<WorkGroup>>())
}


#[test]
fn must_work () {

  const Limit : u64 = 1024;
  struct Ctx { pub counter: AtomicU64 }
  fn bump(tf : TaskFrameHandle) -> ActionPtr {
    let ctx = tf.interpret_frame::<Ctx>();
    let _ = ctx.counter.fetch_add(1, Ordering::Relaxed);
    return ActionPtr::make_completion(false);
  }
  fn swarm_setup_shim(
    _ : TaskFrameHandle, mut handle: TaskGroupHandle
  ) -> ActionPtr {
    for _ in 0 .. Limit {
      let work_item =
        ActionPtr::make_link(LinkKind::Step, bump);
      handle.assign_work(work_item);
    };
    fn checker(tf : TaskFrameHandle) -> Option<ActionPtr> {
      println!("Condition checker pocked!");
      let ctx = tf.interpret_frame::<Ctx>();
      let count = ctx.counter.load(Ordering::Relaxed);
      if count == Limit {
        return Some(ActionPtr::make_link(LinkKind::Step, done));
      }
      return None;
    }
    return ActionPtr::make_progress_checker(checker);
  }
  fn done(tf : TaskFrameHandle) -> ActionPtr {
    println!("you have gazed at miracles!");
    let ctx = tf.interpret_frame::<Ctx>();
    println!("{}", ctx.counter.load(Ordering::Relaxed));
    return ActionPtr::make_completion(true);
  }
  fn begin(tf : TaskFrameHandle) -> ActionPtr {
    let ctx = tf.interpret_frame::<Ctx>();
    ctx.counter = AtomicU64::new(0);
    println!("Greetings!\nWitness the swarm!");
    return ActionPtr::make_fanout(swarm_setup_shim);
  }

  let init =
    ActionPtr::make_link(LinkKind::Step, begin);
  let work_graph =
    ActionPtr::init(
      DataFrameSize::Bytes128, init);

  let start = SystemTime::now();
  let w = WorkGroupRef::init(6, work_graph);
  w.await_completion();
  let finish = SystemTime::now();
  let diff =
    finish.duration_since(start).unwrap();
  println!("Micros : {}", diff.as_micros());

}


//#[test]
fn byte_order () {
  let i =
    PageHeaderData {
      _padding:[0;3], is_detached: true, occupation_map: 1 };
  println!("{:#066b}" , unsafe { transmute::<_, u64>(i) });
  println!("{:#010b}", 0u8 ^ 1 << 2);
  println!( "{}", (!(1u8 << 2)) .trailing_ones() );
}

fn scope () {
  {
    fn func1() {}
  };
  {
    fn func1() {}
  };
}

// #[test]
// fn hhh () {
//   println!("{}", align_of::<[u8;3]>())
// }

#[test]
fn children_see_parrents() {
  struct Ctx { str: String, done: AtomicBool }
  fn step2(tf : TaskFrameHandle) -> ActionPtr { unsafe {
    let pf = tf.get_parrent_frame();
    let parent_frame = &mut *pf.cast::<Ctx>();
    println!("{}", parent_frame.str);
    parent_frame.done.store(true, Ordering::Relaxed);
    return ActionPtr::make_completion(true);
  } }
  fn deleter(_ : TaskFrameHandle) -> ActionPtr {
    return ActionPtr::make_completion(true);
  }
  fn checker(tf : TaskFrameHandle) -> Option<ActionPtr> {
    let frame = tf.interpret_frame::<Ctx>();
    let done = frame.done.load(Ordering::Relaxed);
    if done {
      return Some(
        ActionPtr::make_link(LinkKind::Step, deleter)); }
    return None;
  }
  fn spawn(_ : TaskFrameHandle, mut tg : TaskGroupHandle) -> ActionPtr {
    let p = ActionPtr::make_link(
      LinkKind::Step, step2);
    let p = ActionPtr::init(
        DataFrameSize::Bytes128, p);
    tg.assign_work(p);
    return ActionPtr::make_progress_checker(checker);
  }
  fn step1(tf : TaskFrameHandle) -> ActionPtr { unsafe {
    let frame = tf.interpret_frame::<Ctx>();
    addr_of_mut!(frame.str).write(String::new());
    frame.str.push_str("I do exist!");
    frame.done = AtomicBool::new(false);
    return ActionPtr::make_fanout(spawn);
  } }

  let ptr =
    ActionPtr::make_link(LinkKind::Step, step1);
  let ptr = ActionPtr::init(
    DataFrameSize::Bytes128, ptr);

  let start = SystemTime::now();
  let w = WorkGroupRef::init(1, ptr);
  w.await_completion();
  let finish = SystemTime::now();
  let diff =
    finish.duration_since(start).unwrap();
  println!("Micros : {}", diff.as_micros());

}