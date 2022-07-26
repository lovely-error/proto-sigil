extern crate proto_sigil;
use proto_sigil::support_structures::mini_vector::InlineVector;
use std::mem::size_of;

#[test]
fn inline_vector_stack_behave_reasonably() {
  let mut iv =
    InlineVector::<2, u16>::init();
  iv.push(1);
  iv.push(2);
  if iv.did_allocate_on_heap() {
    panic!("Shoulnd allocate here!")
  }
  assert!(iv.get_ref(0) == &1);
  assert!(iv.get_ref(1) == &2);
  iv.push(3);
  if !iv.did_allocate_on_heap() {
    panic!("Should allocate here!")
  }
  assert!(iv.get_ref(2) == &3);
}

// #[test]
// fn inline_vector_realoc_works_as_designed() {
//   let mut iv =
//     InlineVector::<2, u8>::init();

// }

#[test]
fn size_checks_out() {
  assert!(size_of::<()>() == 0);
  assert!(size_of::<()>() == size_of::<[();0]>());

  let size = size_of::<InlineVector<0, ()>>();
  //println!("{}", size);
  // assert!(size == size_of::<*mut ()>() +
  //                 size_of::<u32>() +
  //                 size_of::<u16>() +
  //                 PADDING);
  assert!(size == 16);
}


#[test]
fn copying_works() {
  let mut iv =
    InlineVector::<2, u16>::init();
  for _ in 0 .. 4 {
    iv.push(u16::MAX);
  }
  assert!(iv.count_items() == 4);
  let mut p : [u16 ; 4] = [0;4];
  iv.move_content_into(p.as_mut_ptr());
  //println!("{:#?}", p);
  assert!(p == [u16::MAX ; 4]);
}

#[test]
fn items_count () {
  let mut iv = InlineVector::<2, u64>::init();
  assert!(iv.count_items() == 0);
  iv.push(0);
  assert!(iv.count_items() == 1);
  iv.push(0);
  assert!(iv.count_items() == 2);
  iv.push(0);
  assert!(iv.count_items() == 3);
}

#[test]
fn put_in_loads () {
  const LIMIT : usize = 10_000;
  let mut iv = InlineVector::<32, usize>::init();
  for i in 0 ..= LIMIT {
    iv.push(i);
  }
  for i in 0 ..= LIMIT {
    assert!(*iv.get_ref(i) == i)
  }
}