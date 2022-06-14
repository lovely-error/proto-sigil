
use std::{time::{SystemTime}, sync::atomic::{AtomicU64, Ordering}};

use proto_sigil::elaborator::{
  action_chain::{
    ActionPtr, LinkKind, DataFrameSize, TaskGroupHandle, TaskFrameHandle },
  worker::{WorkGroupRef}};

fn main () {

  const Limit : u64 = 1024;
  struct Ctx { pub counter: AtomicU64 }
  fn bump(mut tf : TaskFrameHandle) -> ActionPtr {
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
    fn checker(mut tf : TaskFrameHandle) -> Option<ActionPtr> {
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
  fn done(mut tf : TaskFrameHandle) -> ActionPtr {
    println!("you have gazed miracles!");
    let ctx = tf.interpret_frame::<Ctx>();
    println!("{}", ctx.counter.load(Ordering::Relaxed));
    return ActionPtr::make_completion(true);
  }
  fn begin(mut tf : TaskFrameHandle) -> ActionPtr {
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