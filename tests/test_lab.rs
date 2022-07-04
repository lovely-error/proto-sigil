

use std::{
  alloc::dealloc, mem::{size_of,}, time::{SystemTime},
  sync::{atomic::{AtomicU64, Ordering, AtomicBool}}, ptr::{addr_of_mut, null_mut}, hash::{Hash, Hasher}, collections::hash_map::DefaultHasher};

use proto_sigil::{elaborator::{
  action_chain::{
    ActionLink, LinkKind, DataFrameSize, TaskHandle, },
  worker::{WorkGroupRef, WorkGroup},},};

use proto_sigil::{
  support_structures::no_bullshit_closure::DetachedClosure,
  detached,
  build_capture_tuple,
  build_destructor_tuple,
  mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec, };

static mut FLAG : bool = false;

struct Example(bool);
impl Drop for Example {
  fn drop(&mut self) {
    unsafe { FLAG = true };
  }
}

#[test]
fn drop_on_ptrs () {
  use std::alloc::{alloc, Layout};

  unsafe {
    let lay = Layout::new::<Example>();
    let mem_ptr = alloc(lay);
    *mem_ptr.cast::<Example>() = Example(true);
    // this does mean that writing through ptr may drop garbage.
    assert_eq!(FLAG, true);
    //mem_ptr.cast::<Example>().write(Example(true));
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
  fn bump(tf : TaskHandle) -> ActionLink {
    let ctx = tf.interpret_frame::<Ctx>();
    let _ = ctx.counter.fetch_add(1, Ordering::Relaxed);
    return ActionLink::make_completion(false);
  }
  fn done(tf : TaskHandle) -> ActionLink {
    //println!("you have gazed at miracles!");
    let ctx = tf.interpret_frame::<Ctx>();
    //println!("{}", ctx.counter.load(Ordering::Relaxed));
    assert_eq!(ctx.counter.load(Ordering::Relaxed), Limit);
    return ActionLink::make_completion(true);
  }
  fn begin(mut handle : TaskHandle) -> ActionLink {
    let ctx = handle.interpret_frame::<Ctx>();
    ctx.counter = AtomicU64::new(0);
    //println!("Greetings!\nWitness the swarm!");
    for _ in 0 .. Limit {
      let work_item =
        ActionLink::goto(bump);
      handle.assign_work_for_schedule(work_item);
    };
    fn checker(tf : TaskHandle) -> Option<ActionLink> {
      //println!("Condition checker pocked!");
      let ctx = tf.interpret_frame::<Ctx>();
      let count = ctx.counter.load(Ordering::Relaxed);
      if count == Limit {
        return Some(ActionLink::goto( done));
      }
      return None;
    }
    return ActionLink::make_progress_checker(checker);
  }

  let init =
    ActionLink::goto(begin);
  let work_graph =
    ActionLink::make_frame_request(
      DataFrameSize::Bytes120, init);

  // let start = SystemTime::now();
  let w = WorkGroupRef::init(6, work_graph);
  w.await_completion();
  // let finish = SystemTime::now();
  // let diff =
  //   finish.duration_since(start).unwrap();
  // println!("Micros : {}", diff.as_micros());

}


//#[test]
fn byte_order () {
  println!("{:#066b}" , 1);
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
  fn step2(tf : TaskHandle) -> ActionLink {
    let pf = tf.get_parrent_frame().unwrap();
    let parent_frame = pf.interpret_frame::<Ctx>();
    //println!("{}", parent_frame.str);
    assert_eq!(parent_frame.str, "I do exist!");
    parent_frame.done.store(true, Ordering::Relaxed);
    return ActionLink::make_completion(true);
  }
  fn deleter(_ : TaskHandle) -> ActionLink {
    return ActionLink::make_completion(true);
  }
  fn checker(tf : TaskHandle) -> Option<ActionLink> {
    let frame = tf.interpret_frame::<Ctx>();
    let done = frame.done.load(Ordering::Relaxed);
    if done {
      return Some(
        ActionLink::goto( deleter)); }
    return None;
  }
  fn step1(mut tf : TaskHandle) -> ActionLink { unsafe {
    let frame = tf.interpret_frame::<Ctx>();
    addr_of_mut!(frame.str).write(String::new());
    frame.str.push_str("I do exist!");
    frame.done = AtomicBool::new(false);
    let p = ActionLink::goto(step2);
    let p = ActionLink::make_frame_request(
        DataFrameSize::Bytes56, p);
    tf.assign_work_for_schedule(p);
    return ActionLink::make_progress_checker(checker);
  } }

  let ptr =
    ActionLink::goto( step1);
  let ptr = ActionLink::make_frame_request(
    DataFrameSize::Bytes56, ptr);

  // let start = SystemTime::now();
  let w = WorkGroupRef::init(1, ptr);
  w.await_completion();
  // let finish = SystemTime::now();
  // let diff =
    // finish.duration_since(start).unwrap();
  // println!("Micros : {}", diff.as_micros());

}


//#[test]
fn p () {
  let str = "aoao".to_string();
  let str2 = "oaoa".to_string();
  let mut hasher = DefaultHasher::new();
  str.hash(&mut hasher);
  let hash1 = hasher.finish();
  println!("{}", hash1);
  let mut hasher = DefaultHasher::new();
  str2.hash(&mut hasher);
  let hash2 = hasher.finish();
  println!("{}", hash2);

  println!("Rem {}", hash1 % 32);
  println!("Rem {}", hash2 % 32);

}

//#[test]
fn simd () {
  // use std::simd;

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
      return ActionLink::make_completion(true);
    }
    let cont =
      ActionLink::make_gateway(gw.erase_to_sendable());
    let framed = ActionLink::make_frame_request(
      DataFrameSize::Bytes56, cont);
    return framed;
  }

  let task = make_task("yo".to_string());

  let w = WorkGroupRef::init(1, task);
  w.await_completion();

}

#[test]
fn read_zst_from_null () {
  let inv : *mut () = null_mut();
  let () = unsafe { inv.read() };
}

#[test]
fn count_ones () {
  let num = !0u64;
  let count = num.trailing_ones();
  println!("{count}")
}