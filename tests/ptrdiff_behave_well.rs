
use proto_sigil::parser::node_allocator::EntangledPtr;
extern crate proto_sigil;


#[test]
fn diff_ok() {
  let a = 7usize as *mut ();
  let b = 11usize as *mut ();

  let diff =
    EntangledPtr::from_ptr_pair(a, b);
  //println!("{:#?}", diff);
  if let Some(EntangledPtr(diff)) = diff {
    assert!(diff == 4);
    assert!(diff as usize + (a as usize) == 11);
  } else { panic!("Wheres diff??") }

  let diff =
    EntangledPtr::from_ptr_pair(b, a);
  if let Some(EntangledPtr(diff)) = diff {
    assert!(diff == -4);
    assert!(b as isize + (diff as isize) == 7);
  } else { panic!("Wheres diff??") }

  let diff =
    EntangledPtr::from_ptr_pair(a, b);
  if let Some(ptr) = diff {
    let origin =
      ptr.reach_referent_from(a);
    assert!(origin as usize == b as usize);
  } else { panic!("Wheres diff??") }

  let diff =
    EntangledPtr::from_ptr_pair(b, a);
  if let Some(ptr) = diff {
    let origin =
      ptr.reach_referent_from(b);
    assert!(origin as usize == a as usize);
  } else { panic!("Wheres diff??") }
}