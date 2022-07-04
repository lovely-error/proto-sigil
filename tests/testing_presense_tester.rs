
use std::{
  thread::{spawn, park, JoinHandle,},
  ptr::addr_of, mem::MaybeUninit, time::{SystemTime},};

use proto_sigil::elaborator::presense_tester::PresenseSet;



#[test]
fn ping_pong () {
  const Limit : u64 = 50;

  let pt = PresenseSet::<u64>::init();

  let mut thread1 = MaybeUninit::uninit() ;
  let mut thread2 = MaybeUninit::uninit() ;

  let ref1 = addr_of!(pt) as u64;
  let th2 = addr_of!(thread2) as u64;

  thread1.write(spawn(move ||{
    park();
    for i in 0 .. Limit {
      unsafe { (&*(ref1 as *const PresenseSet<u64>)).check_in(&i) };
      unsafe { (&*(th2 as *const JoinHandle<()>)).thread().unpark() };
      park();
    }
  }));

  let ref2 = addr_of!(pt) as u64;
  let th1 = addr_of!(thread1) as u64;

  thread2.write(spawn(move || {
    for i in 0 .. Limit {
      park();
      let was_there =
        unsafe { (&*(ref2 as *const PresenseSet<u64>)).check_out(&i) };
      //println!("{was_there}");
      assert!(was_there);
      unsafe { (&*(th1 as *const JoinHandle<()>)).thread().unpark() };
    }
  }));

  unsafe {
    let th1 = thread1.assume_init();
    let th2 = thread2.assume_init();
    th1.thread().unpark();
    let _ = th1.join();
    let _ = th2.join();
  }

}


#[test]
fn simpleton () {
  static Limit : u64 = 300;
  for k in 0 .. 10 {
    let ps = PresenseSet::<u64>::init();
    for i in 0 .. Limit {
      let was_there = ps.check_in(&i);
      assert!(!was_there);
    }
    for i in 0 .. Limit {
      let was_there = ps.check_out(&i);
      // if !was_there {
      //   println!("Lost item {i} in iteration {k}");
      // }
      assert!(was_there);
    }
  }
}

#[test]
fn pefr () {
  const Limit : u64 = 5000;
  let ps = PresenseSet::<u64>::init();
  let start = SystemTime::now();
  for i in 0 .. Limit {
    let _ = ps.check_in(&i);
  }
  let end = start.elapsed().unwrap().as_millis();
  println!("Spent millis on insertion {end}");

  let start = SystemTime::now();
  for i in 0 .. Limit {
    let _ = ps.check_out(&i);
  }
  let end = start.elapsed().unwrap().as_millis();
  println!("Spent millis on retrieving {end}");
}