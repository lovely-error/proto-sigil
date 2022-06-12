
use std::{mem::size_of, thread::JoinHandle};

use proto_sigil::{trees::raw_syntax_nodes::{
  AppNodeArgsInline, AppNodeVec, LiftNodeItem, ExprPtr},
  parser::parser::symbol::Symbol, elaborator::{worker::LoopQueue,
    action_chain::Task}, preliminaries::mini_vector::InlineVector};

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


// #[test]
// fn huh2 () {
//   let fun =
//     Box::new(|| { println!("yo"); RequestSTM::init_null() });
//   let thing = RequestSTM::init_from_boxed_closure(fun);
//   thing.invoke();
// }

#[test]
fn loop_size () {
  println!("{}", size_of::<LoopQueue<()>>())
}


