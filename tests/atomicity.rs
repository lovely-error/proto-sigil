use std::{thread::{spawn, self}, time::Duration, sync::atomic::{Ordering, AtomicU32, AtomicU64, fence}};

use proto_sigil::support_structures::{atomic_work_queue::{RWData, SyncedPtrs}, universal_bitwise_conversion::bitcast};



#[test]
fn no_torn_writes () {

  // this checks that threads which share atomically synced region of mem
  // do observe writes consistently even if values arent written at exactly
  // same addresses

  for _ in 0 .. 10000 {
    let shared_mutable_state = RWData::init();
    let SyncedPtrs { read_ptr, write_ptr } =
      shared_mutable_state.project_rw_ptr_data();
    let val1;
    let val2;
    let whole;
    let whole2;
    unsafe {
      val1 = bitcast::<_, u64>(read_ptr);
      val2 = bitcast::<_, u64>(write_ptr);
      whole = bitcast::<_, u64>(shared_mutable_state.get_synced_obj());
      whole2 = bitcast::<_, u64>(shared_mutable_state.get_synced_obj());
    };
    let thread1 = spawn(move ||{
      let rp ;
      let whole_;
      unsafe {
        rp = bitcast::<_, &AtomicU32>(val1);
        whole_ = bitcast::<_, &AtomicU64>(whole)
      };
      //let val = rp.load(Ordering::Relaxed);
      let whole_value = whole_.load(Ordering::Relaxed);
      assert!(whole_value != u32::MAX as u64 + 2);
      // println!("Thread1 see {:#X}", whole_value);
      // fence(Ordering::SeqCst);
      rp.store(1, Ordering::Relaxed)
    });
    let thread2 = spawn(move ||{
      let wp ;
      let whole_;
       unsafe {
        wp = bitcast::<_, &AtomicU32>(val2);
        whole_ = bitcast::<_, &AtomicU64>(whole2)
      };
      // let val = wp.load(Ordering::Relaxed);
      let whole_value = whole_.load(Ordering::Relaxed);
      // println!("Thread2 see {:#X}", whole_value);
      // fence(Ordering::SeqCst);
      assert!(whole_value != u32::MAX as u64 + 2);
      wp.store(1, Ordering::Relaxed)
    });
    let _ = thread1.join();
    let _ = thread2.join();
    // println!()
  }

}