use proto_sigil::{
  support_structures::mini_vector::InlineVector, elaborator::worker::LoopQueue};


#[test]
fn qc () {
  let mut iv = InlineVector::<4, u64>::init();
  for i in 510 .. 515 {
    iv.push(i);
  }
  let mut lq = LoopQueue::<u64>::init_new();
  for i in 0 .. 510 {
    lq.enqueue_item(i);
  }
  iv.copy_quickly_into(&mut lq);
  for i in 0 .. 515 {
    assert!(lq.dequeue_item().unwrap() == i)
  }


  let mut iv = InlineVector::<4, u64>::init();
  for i in 0 .. 8 {
    iv.push(i);
  }
  let mut lq = LoopQueue::<u64>::init_new();

  iv.copy_quickly_into(&mut lq);
  for i in 0 .. 8 {
    assert!(*iv.get_ref(i) == i as u64);
  }
}