use std::{time::SystemTime, collections::HashMap, mem::MaybeUninit, thread::{JoinHandle, self}, ptr::addr_of};

use proto_sigil::elaborator::environment::PasteboardTable;


#[test]
fn inout_preservation () {
  // Performance of retrieving in PasteboardTable
  // degrades more when input is too big, if compared to
  // HashMap.
  // Table behave better on insertions since it
  // does not rellocate storage
  const Limit: u64 = 100_000;
  let table =
    PasteboardTable::<u64, u64>::init();
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

//#[test]
fn random_access_performance () {
  // Pasteboard retrieving operation on random keys on
  // not too big amounts has pretty acceptable
  // performance.
  const Limit: u64 = 5000;
  let table =
    PasteboardTable::<u64, u64>::init();
  for i in 0 .. Limit {
    table.insert(&i, i)
  }
  table.freeze();
  let start = SystemTime::now();
  let mut ix : u64 = 8_093_057_678_145_770_544;
  for _ in 0 .. Limit {
    ix = ix.rotate_left(3);
    let random_key = ix % Limit;
    let _ = table.retrieve_ref(&random_key);
  }
  let end = SystemTime::now();
  println!(
    "Table retrieving ended in {} ms",
    end.duration_since(start).unwrap().as_millis());


  let mut map = HashMap::<u64, u64>::new();

  for i in 0 .. Limit {
    map.insert(i, i);
  }

  let start = SystemTime::now();
  for _ in 0 .. Limit {
    ix = ix.rotate_left(3);
    let random_key = ix % Limit;
    let _ = map.get(&random_key);
  }
  let end = SystemTime::now();
  println!(
    "Map retrieving ended in {} ms", end.duration_since(start).unwrap().as_millis());
}

