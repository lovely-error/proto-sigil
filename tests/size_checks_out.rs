
use std::{mem::size_of};

use proto_sigil::{
  expression_trees::{raw_syntax_nodes::{
    AppNodeArgsInline, AppNodeVec, LiftNodeItem, ExprPtr,
  }, better_nodes::{RawNode, ConcretisedNode, }},
  parser::parser::symbol::Symbol,
  elaborator::{
    worker::WorkQueue,
    action_chain::Task
  },
  support_structures::mini_vector::InlineVector
};

#[test]
fn size_of_symbol_is_8_bytes () {
  assert!(size_of::<Symbol>() == 16)
}

#[test]
fn size_of_expr_ptr () {
  assert!(size_of::<ExprPtr>() == 8)
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
  println!("{}", size_of::<WorkQueue<()>>())
}

#[test]
fn task_size () {
  assert_eq!(size_of::<Task>(), 16)
}

#[test]
fn iv_size () {
  assert_eq!(size_of::<InlineVector::<0, ()>>(), 16)
}


#[test]
fn zero_types () {
  assert!(size_of::<(u64, ())>() == size_of::<u64>())
}

#[test]
fn size_of_alternative_raw_node () {
  println!("Size of RawNode is {} bytes", size_of::<RawNode>());
  assert!(size_of::<RawNode>() <= 64);
}

#[test]
fn size_of_checked_node () {
  println!("Size of CheckedNode is {} bytes", size_of::<ConcretisedNode>());
  assert!(size_of::<ConcretisedNode>() <= 64);
}

