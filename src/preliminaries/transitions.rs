use crate::preliminaries::{redefinitions::*, monad::*};


impl <X, Y, I> Transient<X, Either<Y, I>> for Either<Y, X> {
  fn transport(
    self,
    path: impl FnOnce(X) -> Either<Y, I>
  ) -> Either<Y, I> {
    match self {
      Either::Left(err) => Either::Left(err),
      Either::Right(val) => path(val)
    }
  }
}
impl <X, Y, I> Transient<X, Option<I>> for Either<Y, X> {
  fn transport(
    self,
    path: impl FnOnce(X) -> Option<I>
  ) -> Option<I> {
    match self {
      Either::Left(_) => Option::None,
      Either::Right(val) => path(val)
    }
  }
}
impl <X, Y, I> Transient<X, Option<I>> for Result<X, Y> {
  fn transport(
    self,
    path: impl FnOnce(X) -> Option<I>
  ) -> Option<I> {
    match self {
      Result::Err(_) => Option::None,
      Result::Ok(val) => path(val)
    }
  }
}
impl <X, Y, I> Transient<X, Result<I, Y>> for Result<X, Y> {
  fn transport(
    self,
    path: impl FnOnce(X) -> Result<I, Y>
  ) -> Result<I, Y> {
    match self {
      Result::Err(err) => Result::Err(err),
      Result::Ok(val) => path(val)
    }
  }
}
impl <V, K> Transient<V, Option<K>> for Option<V> {
  fn transport(
    self,
    path: impl FnOnce(V) -> Option<K>
  ) -> Option<K> {
    match self {
      Some(val) => path(val),
      None => None,
    }
  }
}
impl <V> Transient<V, ()> for Option<V> {
  fn transport(
    self,
    path: impl FnOnce(V) -> ()
  ) -> () {
    match self {
      Some(val) => path(val),
      None => (),
    }
  }
}