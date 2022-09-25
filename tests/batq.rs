use std::{thread::spawn, sync::atomic::{AtomicU8, Ordering, fence}};

use proto_sigil::{
  support_structures::{
    atomic_work_queue::BoundedAtomicTaskQueue,
    universal_bitwise_conversion::bitcast
  },
  elaborator::action_chain::Task
};



#[test]
fn single_threaded_case () { unsafe {
  let btq = BoundedAtomicTaskQueue::init();

  for i in 0 .. 511u64 {
    let fake = bitcast::<_, Task>(i);
    // println!("{}", bitcast::<_, u64>(fake));
    let succeeded = btq.try_enqueue_task(fake);
    assert!(succeeded);
  }

  for i in 0 .. 511u64 {
    let succeeded = btq.dequeue_task();
    match succeeded {
      Some(task) => {
        let fake = bitcast::<_, u64>(task);
        // println!("{}", fake);
        assert_eq!(fake, i);
      }
      None => {
        panic!("Expected to dequeue task");
      }
    }
  }

} }

#[test]
fn mpsc_case () {
  for _ in 0 .. 10000 {
    let btq = BoundedAtomicTaskQueue::init();

    let ref_ = unsafe { bitcast::<_, u64>(&btq)};
    let handles = [
      spawn(move ||{
        for i in 0 .. 128u64 {
          let ref_ = unsafe { bitcast::<_, &BoundedAtomicTaskQueue>(ref_)};
          let fake = unsafe { bitcast::<_, Task>(i) };
          let succeeded = ref_.try_enqueue_task(fake);
          assert!(succeeded);
        }
      }),
      spawn(move ||{
        for i in 128 .. 256u64 {
          let ref_ = unsafe { bitcast::<_, &BoundedAtomicTaskQueue>(ref_)};
          let fake = unsafe { bitcast::<_, Task>(i) };
          let succeeded = ref_.try_enqueue_task(fake);
          assert!(succeeded);
        }
      }),
      spawn(move ||{
        for i in 256 .. 384u64 {
          let ref_ = unsafe { bitcast::<_, &BoundedAtomicTaskQueue>(ref_)};
          let fake = unsafe { bitcast::<_, Task>(i) };
          let succeeded = ref_.try_enqueue_task(fake);
          assert!(succeeded);
        }
      }),
      spawn(move ||{
        for i in 384 .. 511u64 {
          let ref_ = unsafe { bitcast::<_, &BoundedAtomicTaskQueue>(ref_)};
          let fake = unsafe { bitcast::<_, Task>(i) };
          let succeeded = ref_.try_enqueue_task(fake);
          assert!(succeeded);
        }
      }),
    ];

    for h in handles {
     let _ = h.join().unwrap();
    }

    let mut vec = Vec::<u64>::new();
    for _ in 0 .. 511 {
      let task = btq.dequeue_task();
      vec.push(unsafe { bitcast::<_, u64>(task.unwrap()) } );
    }
    vec.sort();
    for i in 0 .. 511usize {
      assert_eq!(vec[i], i as u64);
    }
  }
}


#[test]
fn mpmc_case () { unsafe {
  let bq = BoundedAtomicTaskQueue::init();
  let flag = AtomicU8::new(0);

  let h = bitcast::<_, u64>(&bq);
  let f = bitcast::<_, u64>(&flag);

  let mut c1 = Vec::<u64>::new();
  let rc1 = bitcast::<_, u64>(&mut c1);
  let mut c2 = Vec::<u64>::new();
  let rc2 = bitcast::<_, u64>(&mut c2);

  let hds = [
    spawn(move ||{
      let h = bitcast::<_, &BoundedAtomicTaskQueue>(h);
      for i in 0 .. 256u64 {
        let fake = bitcast::<_, Task>(i);
        let s = h.try_enqueue_task(fake);
        // assert!(s);
      }
      fence(Ordering::Release);
      bitcast::<_, &AtomicU8>(f).fetch_add(1, Ordering::Relaxed);
    }),
    spawn(move ||{
      let h = bitcast::<_, &BoundedAtomicTaskQueue>(h);
      let f = bitcast::<_, &AtomicU8>(f);
      let c = bitcast::<_, &mut Vec<u64>>(rc1);
      loop {
        let deq = h.dequeue_task();
        if let Some(v) = deq {
          c.push(bitcast(v));
          continue;
        }
        if f.load(Ordering::Relaxed) == 2 {
          break;
        }
      }
    }),
    spawn(move || {
      let h = bitcast::<_, &BoundedAtomicTaskQueue>(h);
      for i in 256 .. 510u64 {
        let fake = bitcast::<_, Task>(i);
        let s = h.try_enqueue_task(fake);
        // assert!(s);
      }
      fence(Ordering::Release);
      bitcast::<_, &AtomicU8>(f).fetch_add(1, Ordering::Relaxed);
    }),
    spawn(move ||{
      let h = bitcast::<_, &BoundedAtomicTaskQueue>(h);
      let f = bitcast::<_, &AtomicU8>(f);
      let c = bitcast::<_, &mut Vec<u64>>(rc2);
      loop {
        let deq = h.dequeue_task();
        if let Some(v) = deq {
          c.push(bitcast(v));
          continue;
        }
        if f.load(Ordering::Relaxed) == 2 {
          break;
        }
      }
    }),
  ];
  for h in hds {
    let _ = h.join().unwrap();
  }


  let mut c3 = Vec::<u64>::new();
  c3.reserve(c1.len() + c2.len());
  c3.append(&mut c1);
  c3.append(&mut c2);
  c3.sort();
  println!("{:#?}", c3);

  for i in 0 .. 510usize {
    let item = c1.get(i).unwrap();
    assert!(*item == i as u64);
  }
} }