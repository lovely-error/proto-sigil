
use std::{str::FromStr, ptr::addr_of_mut};

use proto_sigil::support_structures::tagged_ptr::TaggedPtr;


#[test]
fn encode_decode_ptr () {
  #[repr(packed)]
  #[derive(Copy, Clone, Debug, PartialEq)]
  struct Tag {
    is_cool: bool,
    is_trash: bool,
  }
  type MyPtr<T> = TaggedPtr<Tag, T>;

  let ptr = 1997u64 as *mut ();
  let tag = Tag { is_cool: true, is_trash: false };
  let my_ptr = MyPtr::init_from_ptr(tag, ptr);
  assert!(tag == my_ptr.project_tag());
  assert!(ptr == my_ptr.project_ptr());
}

#[test]
fn ptr_works_less_triv () {
  #[repr(packed)]
  #[derive(Copy, Clone, Debug, PartialEq)]
  struct Tag {
    is_cool: bool,
    degree: u8,
  }
  type MyPtr<T> = TaggedPtr<Tag, T>;

  let text = "Humanity was a mistake";
  let mut str = String::from_str(text).unwrap();
  let ptr = addr_of_mut!(str);
  let tag = Tag { is_cool: true, degree: 0xFF };
  let my_ptr = MyPtr::init_from_ptr(tag, ptr);
  assert!(tag == my_ptr.project_tag());
  let pr_str = unsafe { &*my_ptr.project_ptr() };
  // println!("{}", pr_str);
  assert!(text == pr_str);
}

#[test]
fn ptr_jections () {
  #[derive(Debug, Clone, Copy, PartialEq)]
  struct Flags { flag: bool }
  let mut tptr = TaggedPtr::<Flags, ()>::init_null();
  let fake_ptr = 1997u64 as *mut ();
  tptr.inject_ptr(fake_ptr);
  let flag = Flags { flag: true };
  tptr.inject_tag(flag);
  assert!(tptr.project_tag() == flag);
  assert!(tptr.project_ptr() == fake_ptr);
}

