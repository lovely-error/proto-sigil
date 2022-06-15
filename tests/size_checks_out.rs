
use std::{mem::size_of};

use proto_sigil::{trees::raw_syntax_nodes::{
  AppNodeArgsInline, AppNodeVec, LiftNodeItem, ExprPtr},
  parser::parser::symbol::Symbol, elaborator::{worker::LoopQueue,
    action_chain::Task}, support_structures::mini_vector::InlineVector};

#[test]
fn size_of_symbol_is_8_bytes () {
  assert!(size_of::<Symbol>() == 8)
}

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

#[test]
fn size_of_head_item_check () {
  println!("{}", size_of::<LiftNodeItem>())
}

#[test]
fn size_of_imp_ctx_item () {
  println!("{}", size_of::<(Symbol, Option<ExprPtr>)>());
}

#[test]
fn loop_size () {
  println!("{}", size_of::<LoopQueue<()>>())
}

#[test]
fn task_size () {
  assert_eq!(size_of::<Task>(), 16)
}

#[test]
fn iv_size () {
  assert_eq!(size_of::<InlineVector::<0, ()>>(), 16)
}

// #[test]
// fn th_size () {
//  use std::thread::JoinHandle;
//   println!("{}", size_of::<JoinHandle<()>>())
// }