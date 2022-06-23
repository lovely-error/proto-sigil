
use std::marker::PhantomData;

use super::action_chain::{TaskHandle, TaskGroupHandle};


trait TypeList {
  type Head;
  type Tail: TypeList;
}
struct Cons<A, B: TypeList> { __: PhantomData<(A, B)> }
impl <A, B: TypeList> TypeList for Cons<A, B> {
  type Head = A;
  type Tail = B;
}
enum End {}
impl TypeList for End {
  type Head = Self;
  type Tail = Self;
}
trait TwoTuple {
  type One;
  type Two;
}
// trait TwoTupleHead: TypeList
//   where
//     Self::Head: TwoTuple,
//     Self::Tail: TwoTupleHead  {}

// Task interface
enum ActionChain<Args: TypeList, Result> {

  Step(Args::Head, fn (Args::Head) -> ActionChain<Args::Tail, Result>),

  Fanout(Args::Head, fn (Args::Head) -> ActionChain<Args::Tail, Result>),

  ProgressCheck(fn () -> ActionChain<Args, Result>),

  Completion { should_dispose_frame: bool, outcome: Result },

}

trait STM<C, N> {
  fn advance(self) -> N;
}

impl <Y: TypeList, U>
  STM<ActionChain<Y, U>, ActionChain<Y::Tail, U>>
  for ActionChain<Y, U> {
    fn advance(self) -> ActionChain<Y::Tail, U> {
      match self {
        ActionChain::Step(v, f) => f(v),
        ActionChain::Fanout(v, f) => f(v),
        ActionChain::ProgressCheck(_) => panic!(),
        ActionChain::Completion { .. } => panic!(),
      }
    }
}

// impl <Y: TypeList, U>
//   STM<ActionChain<Y, U>, ActionChain<Y, U>>
//   for ActionChain<Y, U> {

// }

fn make_completion<T>(result: T, should_dispose_frame: bool) -> ActionChain<End, T> {
  ActionChain::Completion { should_dispose_frame, outcome: result }
}
// fn make_intro<T, K: TypeList, P>(fun: fn (T) -> ActionChain<K, P>) -> ActionChain<Cons<T, K>, P> {
//   ActionChain::Intro(fun)
// }
fn make_step<T, K: TypeList, P>(vals: T, fun: fn (T) -> ActionChain<K, P>) -> ActionChain<Cons<T, K>, P> {
  ActionChain::Step(vals,fun)
}
fn make_fanout<T, K: TypeList, P>(vals: T, fun: fn (T) -> ActionChain<K, P>) -> ActionChain<Cons<T, K>, P> {
  ActionChain::Fanout(vals,fun)
}
fn make_progress_check<P, K: TypeList>(fun: fn () -> ActionChain<K, P>) -> ActionChain<K, P> {
  ActionChain::ProgressCheck(fun)
}

fn sample () {
  let _ =
    make_step((), |_| {
      println!("feeling the smell of the monad already?");
      return make_completion((), false);
    });
}