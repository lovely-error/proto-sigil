
use std::{mem::{size_of, ManuallyDrop}, intrinsics::transmute, ptr::addr_of_mut};

use proto_sigil::{
  expression_trees::{raw_syntax_nodes::{LiftNodeItem, ExprPtr,
  }, better_nodes::{RawNode, ConcretisedNode, Symbol, }},

  elaborator::{
    worker::WorkQueue,
    action_chain::{Task, TaskMetadata}
  },
  support_structures::mini_vector::InlineVector, parser::node_allocator::EntagledPtr
};

#[test]
fn size_of_symbol_is_8_bytes () {
  println!("{}", size_of::<Symbol>());
  assert!(size_of::<Symbol>() == 16)
}

#[test]
fn size_of_expr_ptr () {
  assert!(size_of::<ExprPtr>() == 8)
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


#[test]
fn size_of_compact_node () {
  struct Node {
    a: EntagledPtr<Node>,
    b: EntagledPtr<Node>,
    c: EntagledPtr<Node>,
    d: EntagledPtr<Node>,
    kinda_size: u64,
    name: u64,
  }
  println!("Size of Node is {} bytes", size_of::<Node>());

}

#[test]
fn size_of_task_metadata () {
  // println!("Size of task metadata is {} bytes.", size_of::<TaskMetadata>())
  assert!(size_of::<TaskMetadata>() <= 16, "Size of task metadata is bigger then anticipated!!")
}

