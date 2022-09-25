use std::{sync::atomic::{AtomicU64, Ordering, AtomicBool}, ptr::addr_of_mut, time::SystemTime};

use proto_sigil::elaborator::{
  action_chain::{TaskContext, ActionLink,}, worker::WorkGroup, frame_allocator::SlabSize};

use proto_sigil::{
  support_structures::no_bullshit_closure::DetachedClosure,
  detached,
  build_capture_tuple,
  build_destructor_tuple,
  mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec, };

#[test]
fn massive_spawn () {

  const LIMIT : u64 = 100_000;
  struct Ctx { pub counter: AtomicU64, pub start_time: SystemTime }
  fn bump(ctx : TaskContext) -> ActionLink {
    let par = ctx.get_parrent_frame().unwrap();
    let ctx = par.interpret_frame::<Ctx>();
    let _ = ctx.counter.fetch_add(1, Ordering::Relaxed);
    ctx.start_time = SystemTime::now();
    return ActionLink::make_completion();
  }
  fn done(ctx : TaskContext) -> ActionLink {
    let ctx = ctx.interpret_frame::<Ctx>();
    println!("{}", ctx.counter.load(Ordering::Relaxed));
    let time_pased = ctx.start_time.elapsed().unwrap();
    println!("Passed since start: {:?} micros", time_pased.as_micros());
    println!("Passed since start: {:?} nanos", time_pased.as_nanos());
    assert_eq!(ctx.counter.load(Ordering::Relaxed), LIMIT);
    return ActionLink::make_completion();
  }
  fn begin(ctx : TaskContext) -> ActionLink {
    let frame = ctx.interpret_frame::<Ctx>();
    frame.counter = AtomicU64::new(0);
    for _ in 0 .. LIMIT {
      let work_item =
        ActionLink::from_fun(bump);
      let framed = ActionLink::make_frame_request(SlabSize::Bytes64, work_item);
      ctx.assign_work_for_schedule(framed);
    };
    return ActionLink::from_fun(done);
  }

  let init =
    ActionLink::from_fun(begin);
  let work_graph =
    ActionLink::make_frame_request(
      SlabSize::Bytes128, init);

  // let start = SystemTime::now();
  let w = WorkGroup::init(work_graph);
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
    detached!([str] | tf:TaskContext | {
      let frame = tf.interpret_frame::<Ctx>();
      unsafe { addr_of_mut!(frame.str).write(str) };
      return ActionLink::from_fun( read);
    });
    fn read(tf : TaskContext) -> ActionLink {
      let frame = tf.interpret_frame::<Ctx>();
      println!("{}", frame.str);
      assert!(frame.str == "yo");
      return ActionLink::make_completion();
    }
    let cont =
      ActionLink::make_gateway(gw.erase_to_sendable());
    let framed = ActionLink::make_frame_request(
      SlabSize::Bytes128, cont);
    return framed;
  }

  let task = make_task("yo".to_string());

  let w = WorkGroup::init(task);
  w.await_completion();

}


#[test]
fn children_see_parrents() {
  struct Ctx { str: String, done: AtomicBool }
  fn step2(tf : TaskContext) -> ActionLink {
    let pf = tf.get_parrent_frame().unwrap();
    let parent_frame = pf.interpret_frame::<Ctx>();
    //println!("{}", parent_frame.str);
    assert_eq!(parent_frame.str, "I do exist!");
    parent_frame.done.store(true, Ordering::Relaxed);
    return ActionLink::make_completion();
  }
  fn step1(tf : TaskContext) -> ActionLink { unsafe {
    let frame = tf.interpret_frame::<Ctx>();
    addr_of_mut!(frame.str).write(String::new());
    frame.str.push_str("I do exist!");
    frame.done = AtomicBool::new(false);

    let p = ActionLink::from_fun(step2);
    let p = ActionLink::make_frame_request(
        SlabSize::Bytes128, p);
    tf.assign_work_for_schedule(p);

    return p;
  } }

  let ptr =
    ActionLink::from_fun( step1);
  let ptr = ActionLink::make_frame_request(
    SlabSize::Bytes128, ptr);

  // let start = SystemTime::now();
  let w = WorkGroup::init(ptr);
  w.await_completion();
  // let finish = SystemTime::now();
  // let diff =
    // finish.duration_since(start).unwrap();
  // println!("Micros : {}", diff.as_micros());

}

#[test]
fn well_sequenced_loops () {
  static mut VAL : u8 = 0;
  static mut MSG : String = String::new();
  fn loop_ (ctx: TaskContext) -> ActionLink {
    unsafe { MSG.push_str("Tick")};
    ctx.assign_work_for_schedule(ActionLink::make_task_local_closure(&ctx, (), |_, _| {
      unsafe {
        MSG.push_str("Tock");
        VAL += 1;
      };
      return ActionLink::make_completion();
    }));
    if unsafe { VAL == 5 } {
      return ActionLink::from_fun(end);
    } else {
      return ActionLink::from_fun(loop_);
    }
  }
  fn end (_ : TaskContext) -> ActionLink {
    unsafe {
      MSG.push_str("BOOM!!");
    };
    return ActionLink::make_completion();
  }

  let ptr =
    ActionLink::from_fun(loop_);
  let memed =
  ActionLink::make_autosized_frame_request::<()>(ptr);
  let exec = WorkGroup::init(memed);
  exec.await_completion();

  assert!(
    "TickTockTickTockTickTockTickTockTickTockTickTockBOOM!!" ==
    unsafe { &MSG })
}