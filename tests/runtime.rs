use std::{sync::atomic::{AtomicU64, Ordering, AtomicBool}, ptr::addr_of_mut};

use proto_sigil::elaborator::{action_chain::{TaskHandle, ActionLink, DataFrameSize}, worker::WorkGroupRef};

use proto_sigil::{
  support_structures::no_bullshit_closure::DetachedClosure,
  detached,
  build_capture_tuple,
  build_destructor_tuple,
  mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec, };

#[test]
fn massive_spawn () {

  const LIMIT : u64 = 100_000;
  struct Ctx { pub counter: AtomicU64 }
  fn bump(tf : TaskHandle) -> ActionLink {
    let par = tf.get_parrent_frame().unwrap();
    let ctx = par.interpret_frame::<Ctx>();
    let _ = ctx.counter.fetch_add(1, Ordering::Relaxed);
    return ActionLink::make_completion();
  }
  fn done(tf : TaskHandle) -> ActionLink {
    let ctx = tf.interpret_frame::<Ctx>();
    println!("{}", ctx.counter.load(Ordering::Relaxed));
    assert_eq!(ctx.counter.load(Ordering::Relaxed), LIMIT);
    return ActionLink::make_completion();
  }
  fn begin(handle : TaskHandle) -> ActionLink {
    let ctx = handle.interpret_frame::<Ctx>();
    ctx.counter = AtomicU64::new(0);
    for _ in 0 .. LIMIT {
      let work_item =
        ActionLink::goto(bump);
      let framed = ActionLink::make_frame_request(DataFrameSize::AproxBytes64, work_item);
      handle.assign_work_for_schedule(framed);
    };
    return ActionLink::goto(done);
  }

  let init =
    ActionLink::goto(begin);
  let work_graph =
    ActionLink::make_frame_request(
      DataFrameSize::AproxBytes128, init);

  // let start = SystemTime::now();
  let w = WorkGroupRef::init(6, work_graph);
  w.await_completion();
  // let finish = SystemTime::now();
  // let diff =
  //   finish.duration_since(start).unwrap();
  // println!("Micros : {}", diff.as_micros());

}

#[test]
fn gateway_is_ok () {
  struct Ctx { str: String }
  fn make_task(str: String) -> ActionLink {
    let gw =
    detached!([str] | tf:TaskHandle | {
      let frame = tf.interpret_frame::<Ctx>();
      unsafe { addr_of_mut!(frame.str).write(str) };
      return ActionLink::goto( read);
    });
    fn read(tf : TaskHandle) -> ActionLink {
      let frame = tf.interpret_frame::<Ctx>();
      println!("{}", frame.str);
      assert!(frame.str == "yo");
      return ActionLink::make_completion();
    }
    let cont =
      ActionLink::make_gateway(gw.erase_to_sendable());
    let framed = ActionLink::make_frame_request(
      DataFrameSize::AproxBytes64, cont);
    return framed;
  }

  let task = make_task("yo".to_string());

  let w = WorkGroupRef::init(1, task);
  w.await_completion();

}


#[test]
fn children_see_parrents() {
  struct Ctx { str: String, done: AtomicBool }
  fn step2(tf : TaskHandle) -> ActionLink {
    let pf = tf.get_parrent_frame().unwrap();
    let parent_frame = pf.interpret_frame::<Ctx>();
    //println!("{}", parent_frame.str);
    assert_eq!(parent_frame.str, "I do exist!");
    parent_frame.done.store(true, Ordering::Relaxed);
    return ActionLink::make_completion();
  }
  fn step1(tf : TaskHandle) -> ActionLink { unsafe {
    let frame = tf.interpret_frame::<Ctx>();
    addr_of_mut!(frame.str).write(String::new());
    frame.str.push_str("I do exist!");
    frame.done = AtomicBool::new(false);

    let p = ActionLink::goto(step2);
    let p = ActionLink::make_frame_request(
        DataFrameSize::AproxBytes64, p);
    tf.assign_work_for_schedule(p);

    return p;
  } }

  let ptr =
    ActionLink::goto( step1);
  let ptr = ActionLink::make_frame_request(
    DataFrameSize::AproxBytes64, ptr);

  // let start = SystemTime::now();
  let w = WorkGroupRef::init(1, ptr);
  w.await_completion();
  // let finish = SystemTime::now();
  // let diff =
    // finish.duration_since(start).unwrap();
  // println!("Micros : {}", diff.as_micros());

}
