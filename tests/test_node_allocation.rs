extern crate proto_sigil;


use proto_sigil::parser::node_allocator::{
  LinearAllocator, NodeSlabSizeInBytes};


#[test]
fn alloc_happens_at_all() { unsafe {
  let mut alloc =
    LinearAllocator::<NodeSlabSizeInBytes>::init();
  //println!("{:#?}", alloc);
  assert!(alloc.current_page == alloc.first_page);
  assert!(alloc.ptr == 1);

  alloc.get_slot().cast::<usize>().write(usize::MAX);
  let same_thing =
    *alloc.current_page
      .cast::<[u8;NodeSlabSizeInBytes]>().add(1)
      .cast::<usize>();
  assert!(same_thing == usize::MAX);
  assert!(alloc.ptr == 2);
  //println!("{:#?}", alloc);
} }

#[test]
fn usable() {
  let mut alloc =
    LinearAllocator::<NodeSlabSizeInBytes>::init();
  for _ in 1 .. NodeSlabSizeInBytes - 1 {
    let _ = alloc.get_slot();
  }
  //println!("{:#?}", alloc);
  assert!(alloc.current_page == alloc.first_page);
  let _ = alloc.get_slot();
  assert!(alloc.current_page != alloc.first_page);
}

#[test]
fn can_alloc_contiguos() {
  let mut alloc =
    LinearAllocator::<16>::init();
  let vals = [u64::MAX ; 5];
  //println!("{:?}", alloc);
  let mem = alloc.get_contiguos_mem_for::<[u64;5]>();
  //println!("{:?}", alloc);
  assert!(alloc.ptr == 4);
  unsafe { mem.write(vals) };
  let vals_ = unsafe { mem.read() };
  assert!(vals == vals_)
}

#[test]
fn behaves_well_at_the_boudnry() {
  let mut alloc =
    LinearAllocator::<16>::init();
  for _ in 0 .. 254 {
    let _ = alloc.get_slot();
  }
  assert!(alloc.ptr == 255);
  let page = alloc.current_page;
  let vals = [u64::MAX ; 3];
  unsafe {
    let mem = alloc.get_contiguos_mem_for::<[u64;3]>();
    let page_ = alloc.current_page;
    assert!(page != page_);
    assert!(alloc.ptr == 3);
    mem.write(vals);
    let vals_ = mem.read();
    assert!(vals == vals_)
  };
}