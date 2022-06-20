use std::{
  time::{SystemTime,}, collections::HashMap,
  thread::{spawn}, ptr::addr_of, sync::Mutex};

use proto_sigil::elaborator::environment::PasteboardTable;


#[test]
fn dead_simple_test () {
  let table =
    PasteboardTable::<u64, u64>::init();
  table.insert(&3, 17);
  table.freeze();
  let ret = table.retrieve_ref(&3);
  assert!(*ret.unwrap() == 17);
}

#[test]
fn inout_preservation () {
  // Performance of retrieving in PasteboardTable
  // degrades more when input is too big, if compared to
  // HashMap.
  // Table behave better on insertions since it
  // does not rellocate storage
  const Limit: u64 = 5000;
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

#[test]
fn concurent_insertions () { unsafe {
  for k in 0 .. 10000 {
    let table =
      PasteboardTable::<u64, u64>::init();

    //let start = SystemTime::now();
    let threads = [
      {
        let ref_copy = addr_of!(table) as u64;
        spawn(move ||{
          for i in 0 .. 100u64 {
            (&*(ref_copy as *mut PasteboardTable<u64, u64>))
            .insert(&i, i);
          }
        })
      },
      {
        let ref_copy = addr_of!(table) as u64;
        spawn(move ||{
          for i in 100 .. 200u64 {
            (&*(ref_copy as *mut PasteboardTable<u64, u64>))
            .insert(&i, i);
          }
        })
      },
      {
        let ref_copy = addr_of!(table) as u64;
        spawn(move ||{
          for i in 200 .. 300u64 {
            (&*(ref_copy as *mut PasteboardTable<u64, u64>))
            .insert(&i, i);
          }
        })
      },
      {
        let ref_copy = addr_of!(table) as u64;
        spawn(move ||{
          for i in 300 .. 400u64 {
            (&*(ref_copy as *mut PasteboardTable<u64, u64>))
            .insert(&i, i);
          }
        })
      }
    ];

    for thread in threads.into_iter() {
      let _ = thread.join().unwrap();
    }
    // println!(
    //   "Writing done in {} micros",
    //   start.elapsed().unwrap().as_micros());

    table.freeze();

    let mut vec = Vec::<u64>::new();
    vec.reserve(400);
    for i in 0 .. 400 {
      let item = table.retrieve_ref(&i).unwrap();
      vec.push(*item);
    }
    vec.sort();
    //println!("{:#?}", vec)
    for i in 0 .. 400u64 {
      let item = vec.get(i as usize).unwrap();
      assert!(i == *item);
    }
  }
} }

//#[test]
fn mutexed_hashmap_assessment () {
  let map = HashMap::<u64, u64>::new();
  let map = Mutex::new(map);

  let start = SystemTime::now();
  let threads = [
    {
      let ref_copy = addr_of!(map) as u64;
      spawn(move ||{
        for i in 0 .. 100u64 {
          unsafe {
            let mut map =
              (*(ref_copy as *mut Mutex<HashMap<u64, u64>>)) .lock().unwrap();
            map.insert(i, i);
          };
        }
      })
    },
    {
      let ref_copy = addr_of!(map) as u64;
      spawn(move ||{
        for i in 0 .. 100u64 {
          unsafe {
            let mut map =
              (*(ref_copy as *mut Mutex<HashMap<u64, u64>>)) .lock().unwrap();
            map.insert(i, i);
          };
        }
      })
    },
    {
      let ref_copy = addr_of!(map) as u64;
      spawn(move ||{
        for i in 0 .. 100u64 {
          unsafe {
            let mut map =
              (*(ref_copy as *mut Mutex<HashMap<u64, u64>>)) .lock().unwrap();
            map.insert(i, i);
          };
        }
      })
    },
    {
      let ref_copy = addr_of!(map) as u64;
      spawn(move ||{
        for i in 0 .. 100u64 {
          unsafe {
            let mut map =
              (*(ref_copy as *mut Mutex<HashMap<u64, u64>>)) .lock().unwrap();
            map.insert(i, i);
          };
        }
      })
    },
  ];

  for thread in threads.into_iter() {
    let _ = thread.join().unwrap();
  }
  println!(
      "Writing done in {} micros",
      start.elapsed().unwrap().as_micros());

}

#[test]
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


static mut stunt_drop_count : u16 = 0;
struct DropStunt(bool);
impl Drop for DropStunt {
  fn drop(&mut self) {
    unsafe { stunt_drop_count += 1 }
  }
}

#[test]
fn drops_correctly () {

  const Limit : u16 = 57;

  let table =
    PasteboardTable::<u16, DropStunt>::init();

  for i in 0 .. Limit {
    table.insert(&i, DropStunt(false))
  }

  drop(table);

  assert!(unsafe { stunt_drop_count } == Limit);

}

//#[test]
fn compression_benefits_assessment () {

  static Limit : u64 = 5000;

  let table =
    PasteboardTable::<u64, u64>::init();

  for i in 0 .. Limit {
    table.insert(&i, i);
  }

  let start = SystemTime::now();
  let mut ix : u64 = 8_093_057_678_145_770_544;
  for _ in 0 .. Limit {
    ix = ix.rotate_left(3);
    let random_key = ix % Limit;
    let _ = table.retrieve_ref(&random_key);
  }
  let time = start.elapsed().unwrap().as_millis();
  println!("Lookup when nonfrozen took {time} ms");

  table.freeze();

  let start = SystemTime::now();
  for _ in 0 .. Limit {
    ix = ix.rotate_left(3);
    let random_key = ix % Limit;
    let _ = table.retrieve_ref(&random_key);
  }
  let time = start.elapsed().unwrap().as_millis();
  println!("Lookup when frozen took {time} ms");
}