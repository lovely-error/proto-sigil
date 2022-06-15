
use crate::support_structures::redefinitions::{Either};

pub trait Failable<Failure> {
  fn init_with_failure(failure: Failure) -> Self;
}
pub trait ConstructibleFrom<T> {
  fn init_from_value(value:T) -> Self;
}

pub trait Transient<Focus, Exodus> {
  fn transport(
    self,
    path: impl FnOnce(Focus) -> Exodus
  ) -> Exodus;
}

pub fn fail_with_aux_gen_ctx_intro <E, FM: Failable<E>>
  (failure: E) -> FM {
    FM::init_with_failure(failure)
}
pub fn init_with_aux_gen_ctx_intro <V, CM: ConstructibleFrom<V>>
  (value: V) -> CM {
    CM::init_from_value(value)
}

#[macro_export]
macro_rules! monic {
  ($($tokens:tt)*) => {
    (||{
      monic_unfolding_loop! { $($tokens)* }
    })()
  };
}
#[macro_export]
macro_rules! monic_unfolding_loop {
  ($stmt:stmt ; $($tail:tt)*) => {
    $stmt
    monic_unfolding_loop! { $($tail)* }
  };
  (let $binder:pat in $expr:expr ; $($tail:tt)*) => {
    return $expr.transport(|$binder| {
      monic_unfolding_loop! { $($tail)* }
    });
  };
  () => {};
}

// #[macro_export]
// macro_rules! bind {
//   ($binder:pat in $expr:expr ; $($tail:tt)*) => {

//   };
//   () => {};
// }

#[macro_export]
macro_rules! throw {
  ($expr:expr) => {
    return fail_with_aux_gen_ctx_intro($expr)
  };
  () => {
    return fail_with_aux_gen_ctx_intro(())
  }
}

#[macro_export]
macro_rules! guard {
  ($cond:expr => $fail:expr) => {
    if !$cond {
      return fail_with_aux_gen_ctx_intro($fail);
    }
  }
}

#[macro_export]
macro_rules! emit {
  ($value:expr) => {
    return init_with_aux_gen_ctx_intro($value)
  }
}


impl <T> Failable<()> for Option<T> {
  fn init_with_failure(_failure: ()) -> Option<T> {
    None
  }
}
impl <E, V> Failable<E> for Either<E, V> {
  fn init_with_failure(failure: E) -> Self {
    Either::Left(failure)
  }
}
impl <E, V> Failable<E> for Result<V, E> {
  fn init_with_failure(failure: E) -> Self {
    Result::Err(failure)
  }
}
impl <T> ConstructibleFrom<T> for Option<T> {
  fn init_from_value(value: T) -> Self {
    Some(value)
  }
}
impl <V, E> ConstructibleFrom<V> for Either<E, V> {
  fn init_from_value(value: V) -> Self {
    Either::Right(value)
  }
}
impl <V, E> ConstructibleFrom<V> for Result<V, E> {
  fn init_from_value(value: V) -> Self {
    Result::Ok(value)
  }
}

