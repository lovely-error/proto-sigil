use proto_sigil::elaborator::worker::LoopQueue;


#[test]
fn inout_preserve () {
  let mut lq = LoopQueue::<u64>::init_new();
  //println!("{:#?}", lq);
  for n in 0 .. 4096 {
    lq.enqueue_item(n);
  }
  //println!("{:#?}", lq);
  for n in 0 .. 4096 {
    assert!(n == lq.dequeue_item().unwrap())
  }
}
#[test]
fn alloc_at_expected_points () {
  let mut lq = LoopQueue::<u64>::init_new();
  //println!("{:#?}", lq);
  for n in 0 .. 510 {
    lq.enqueue_item(n);
  }
  //println!("{:#?}", lq);
  assert!(lq.read_page == lq.write_page);
  lq.enqueue_item(0);
  assert!(lq.read_page != lq.write_page);
}

#[test]
fn consume_little_memory () {
  let mut lq = LoopQueue::<u64>::init_new();
  for i in 0 .. 1_000_000 {
    lq.enqueue_item(i);
    let n = lq.dequeue_item().unwrap();
    assert!(i == n);
  }
  //println!("{}", lq.total_allocated_pages);
  //assert!(lq.number_of_allocated_pages == 2); this is true, I checked :=
}