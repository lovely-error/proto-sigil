
use std::mem::size_of;

use proto_sigil::trees::raw_syntax_nodes::{AppNodeArgsInline, AppNodeVec};


#[test]
fn size_test1 () {
  let size = size_of::<AppNodeArgsInline>();
  assert!(size <= 64);
  println!("{}", size);
}


#[test]
fn size_test2 () {
  let size = size_of::<AppNodeVec>();
  assert!(size <= 64);
  println!("{}", size);
}

