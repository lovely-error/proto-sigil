use std::{time::SystemTime, collections::HashMap, mem::MaybeUninit, thread::{JoinHandle, self}, ptr::addr_of};

use proto_sigil::elaborator::environment::PersistentTable;


#[test]
fn inout_preservation () {
  const Limit: u64 = 5000;
  let table =
    PersistentTable::<u64, u64>::init();
  let start = SystemTime::now();
  for i in 0 .. Limit {
    table.insert(&i, i)
  }
  let end = SystemTime::now();
  println!(
    "Table insertions ended in {} ms",
    end.duration_since(start).unwrap().as_millis());
  table.freeze();
  let start = SystemTime::now();
  for i in 0 .. Limit {
    let smth = table.retrieve_ref(&i);
    assert!(*smth.unwrap() == i);
  }
  let end = SystemTime::now();
  println!(
    "Table retrieving ended in {} ms",
    end.duration_since(start).unwrap().as_millis());


  let mut map = HashMap::<u64, u64>::new();

  let start = SystemTime::now();
  for i in 0 .. Limit {
    map.insert(i, i);
  }
  let end = SystemTime::now();
  println!(
    "Map insertions ended in {} ms",
    end.duration_since(start).unwrap().as_millis());

  let start = SystemTime::now();
  for i in 0 .. Limit {
    let smth = map.get(&i);
    assert!(*smth.unwrap() == i);
  }
  let end = SystemTime::now();
  println!(
    "Map retrieving ended in {} ms", end.duration_since(start).unwrap().as_millis());
}

// #[test]
// fn concurent_writing () { unsafe {
//   let table =
//     PersistentTable::<u64, u64>::init();
//   let table_ref = addr_of!(table) as u64;

//   let mut th: [MaybeUninit<JoinHandle<()>> ; 4] =
//     MaybeUninit::uninit().assume_init();
//   for i in 0 .. 4 {
//     let copy = table_ref;
//     let thread = thread::spawn(move || {
//       for i in 0 .. 100 {
//         (&*(copy as *const PersistentTable<u64, u64>))
//         .insert(&i, i);
//       }
//     });
//     th.as_mut_ptr().add(i).cast::<JoinHandle<()>>().write(thread);
//   }
//   for thread in th.into_iter() {
//     let th = thread.assume_init();
//     let _ = th.join().unwrap();
//   }
//   table.freeze();
//   let mut vec = Vec::<u64>::new();
//   vec.reserve(400);
//   for i in 0 .. 400 {
//     let item = table.retrieve_ref(&i).unwrap();
//     vec.push(*item);
//   }
//   vec.sort();
//   for i in 0 .. 400 {
//     let item = vec.get(i).unwrap();
//     assert!(i == *item as usize);
//   }
// } }