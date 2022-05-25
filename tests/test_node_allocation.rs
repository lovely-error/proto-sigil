extern crate proto_sigil;

use proto_sigil::parser::node_allocator::{
  Pager, NodeSizeInBytes};


#[test]
fn alloc_happens_at_all() { unsafe {
  let mut alloc =
    Pager::<NodeSizeInBytes>::init();
  //println!("{:#?}", alloc);
  assert!(alloc.current_page == alloc.last_page);
  assert!(alloc.ptr == 1);

  alloc.get_slot().cast::<usize>().write(usize::MAX);
  let same_thing =
    *alloc.current_page
      .cast::<[u8;NodeSizeInBytes]>().add(1)
      .cast::<usize>();
  assert!(same_thing == usize::MAX);
  assert!(alloc.ptr == 2);
  //println!("{:#?}", alloc);
} }

#[test]
fn usable() {
  let mut alloc =
    Pager::<NodeSizeInBytes>::init();
  for _ in 1 .. NodeSizeInBytes - 1 {
    let _ = alloc.get_slot();
  }
  //println!("{:#?}", alloc);
  assert!(alloc.current_page == alloc.last_page);
  let _ = alloc.get_slot();
  assert!(alloc.current_page != alloc.last_page);
}