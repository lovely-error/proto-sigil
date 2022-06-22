
use std::{
  mem::{size_of, align_of, }, ptr::{null_mut, drop_in_place,},
  alloc::{alloc, Layout, dealloc}, marker::PhantomData,
  intrinsics::{transmute,},};


#[repr(align(16))]
pub struct Closure<Env, I, O>(u64,u64, PhantomData<(Env, I, O)>);

impl <X, Y, I> Closure<X, Y, I> {
  fn project_env_ptr(&self) -> *mut () {
    (self.0 >> 1) as *mut ()
  }
  fn project_action_ptr(&self) -> *mut () {
    (self.1 & ((1 << 40) - 1)) as *mut ()
  }
  fn project_destructor_ptr(&self) -> *mut () {
    let base = (self.1 & ((1 << 40) - 1)) as isize;
    let offset = (self.1 >> 40) as isize;
    let dctor_ptr = (base + offset) as usize;
    return dctor_ptr as *mut ();
  }
  fn was_invoked(&self) -> bool {
    (self.0 & 1) == 1
  }
  fn dont_have_env(&self) -> bool {
    self.project_env_ptr().is_null()
  }
  fn mark_as_invoked(&mut self) {
    self.0 += 1;
  }
}

fn common_invoke_consume_impl<I, O>(
  mut target: CommonClosureRepr, args:I
) -> O { unsafe {
  let action_ptr = target.project_action_ptr();
  let fun =
    transmute::<_, fn (*mut (), I) -> O>(action_ptr);
  if target.dont_have_env() { // trivial closure
    let res = (fun)(null_mut(), args);
    return res;
  } else {
    let env_ptr = target.project_env_ptr();
    let res = (fun)(env_ptr, args);
    target.mark_as_invoked();
    drop(target);
    return res;
  }
} }
fn common_drop_impl(target: &mut CommonClosureRepr) { unsafe {
  if target.dont_have_env() { return; }
  let dctor_ptr = target.project_destructor_ptr();
  let dctor =
    transmute::<_, fn (*mut (), bool)>(dctor_ptr);
  let env_ptr = target.project_env_ptr();
  let need_drop = !target.was_invoked();
  (dctor)(env_ptr, need_drop);
} }

impl <X, Y, I> Closure<X, Y, I> {
  fn dctor(env_ptr: *mut (), need_env_drop: bool) { unsafe {
    if need_env_drop {
      drop_in_place(env_ptr.cast::<X>())
    }
    dealloc(
      env_ptr.cast::<u8>(),
      Layout::from_size_align_unchecked(
        size_of::<X>(), align_of::<X>()));
  } }
  pub fn init_with_given_mem(
    mem: *mut (), env: X, fun: fn (*mut X, Y) -> I
  ) -> Self { unsafe {
    mem.cast::<X>().write(env);
    let fun_ = fun as *mut u8;
    let dctor = Self::dctor as *mut u8;
    let ptr_diff = dctor.offset_from(fun_);
    let mem = (mem as u64) << 1;
    let procs = (fun as u64) + (ptr_diff) as u64;
    return Self(mem, procs, PhantomData)
  } }
  pub fn init_with_global_mem(
    env: X, fun: fn (*mut X, Y) -> I
  ) -> Self { unsafe {
    if size_of::<X>() != 0 {
      let mem = alloc(Layout::from_size_align_unchecked(
        size_of::<X>(), align_of::<X>()));
      mem.cast::<X>().write(env);
      let fun_ = fun as *mut u8;
      let dctor = Self::dctor as *mut u8;
      let ptr_diff = dctor.offset_from(fun_);
      let ptr_diff = (ptr_diff << 40) as u64;
      let mem = (mem as u64) << 1;
      let procs = (fun as u64) + ptr_diff;
      return Self(mem, procs, PhantomData)
    } else {
      return Self(0, fun as u64, PhantomData)
    };
  }; }
  pub fn invoke_consume(self, args: Y) -> I { unsafe {
    let common_repr =
      transmute::<_, CommonClosureRepr>(self);
    common_invoke_consume_impl(common_repr, args)
  } }
}

impl <X, Y, I> Drop for Closure<X, Y, I> {
  fn drop(&mut self) { unsafe {
    let common_repr =
      transmute::<_, &mut CommonClosureRepr>(&mut *self);
    common_drop_impl(common_repr);
  } }
}


impl <X, Y, I> Clone for Closure<X, Y, I> where X:Clone {
  fn clone(&self) -> Self { unsafe {
    if self.dont_have_env() { // just copy bytes
      let copy =
        transmute::<_, *const Self>(self).read();
      return copy;
    } else { // need to clone env
      let new_env_mem =
        alloc(Layout::from_size_align_unchecked(
          size_of::<X>(), align_of::<X>()));
      let env_ptr = self.project_env_ptr();
      let copy = (&*env_ptr.cast::<X>()).clone();
      new_env_mem.cast::<X>().write(copy);
      let mem = (new_env_mem as u64) << 1;
      return Self(mem , self.1, PhantomData)
    }
  } }
}

unsafe impl <X, Y, I> Send for Closure<X, Y, I> where X:Send {}
unsafe impl <X, Y, I> Sync for Closure<X, Y, I> where X:Sync {}

struct CommonClosureRepr(u64,u64);
impl CommonClosureRepr {
  fn project_env_ptr(&self) -> *mut () {
    (self.0 >> 1) as *mut ()
  }
  fn project_action_ptr(&self) -> *mut () {
    (self.1 & ((1 << 40) - 1)) as *mut ()
  }
  fn project_destructor_ptr(&self) -> *mut () {
    let base = (self.1 & ((1 << 40) - 1)) as isize;
    let offset = (self.1 >> 40) as isize;
    let dctor_ptr = (base + offset) as usize;
    return dctor_ptr as *mut ();
  }
  fn was_invoked(&self) -> bool {
    (self.0 & 1) == 1
  }
  fn dont_have_env(&self) -> bool {
    self.project_env_ptr().is_null()
  }
  fn mark_as_invoked(&mut self) {
    self.0 += 1;
  }
}
impl Drop for CommonClosureRepr {
  fn drop(&mut self) {
    common_drop_impl(self)
  }
}

pub struct SomeClosure<I, O>(u64, u64, PhantomData<(I, O)>);

impl <I, O> SomeClosure<I, O> {
  pub fn try_invoke(&mut self, args: I) -> Option<O> { unsafe {
    let shrep =
      transmute::<_, &mut CommonClosureRepr>(&mut *self);
    let no_env = shrep.dont_have_env();
    if !no_env && shrep.was_invoked() { // this clos is one-shot
      return None;
    }
    let action_ptr = shrep.project_action_ptr();
    let fun =
      transmute::<_, fn (*mut (), I) -> O>(action_ptr);
    if no_env { // trivial closure
      let res = (fun)(null_mut(), args);
      return Some(res);
    } else {
      let env_ptr = shrep.project_env_ptr();
      let res = (fun)(env_ptr, args);
      shrep.mark_as_invoked();
      return Some(res);
    }
  }; }
}

impl <X, Y, I> Closure<X, Y, I> {
  pub fn erase_to_some(self) -> SomeClosure<Y, I> {
    unsafe { transmute(self) }
  }
}

pub struct SomeSendableClosure<I, O>(u64,u64, PhantomData<(I, O)>);

impl <I, O> SomeSendableClosure<I, O> {
  pub fn erase_to_some(self) -> SomeClosure<I, O> {
    unsafe { transmute(self) }
  }
  pub fn invoke_consume(self, args: I) -> O {
    let common_repr = unsafe { transmute(self) };
    return common_invoke_consume_impl(common_repr, args)
  }
}

unsafe impl <I, O> Send for SomeSendableClosure<I, O> {}

impl <X, Y, I> Closure<X, Y, I> where X:Send {
  pub fn erase_to_sendable(self) -> SomeSendableClosure<Y, I> {
    unsafe { transmute(self) }
  }
}


#[macro_export]
macro_rules! closure {
  ( [ $($capt_name:ident $(= $expr:expr)?),* ]
    $(| $( $arg_name:ident $(: $ty:ty)? ),* |)?
    $(-> $rt:ty)? $bl:block )
  => {
    {
      let env = build_capture_tuple! { $($capt_name $(= $expr)? ,)* };
      let clos = Closure::init_with_global_mem(
        env, | env , build_arg_destructor_tuple! { $($($arg_name ,)*)? }
          : mk_ty_rec! { $($($arg_name $(: $ty)? ,)*)? }
        |
        $(-> $rt)? {
          let build_destructor_tuple! { $($capt_name $(= $expr)? ,)* }
            = unsafe { env.read() };
          $bl
        });
      clos
    }
  };
}
#[macro_export]
macro_rules! mk_ty_rec {
  ($_:ident : $ty:ty , $($tail:tt)*) => {
    ( $ty , mk_ty_rec! { $($tail)* } )
  };
  ($_:ident , $($tail:tt)*) => {
    ( _ , mk_ty_rec! { $($tail)* } )
  };
  () => { () };
}
macro_rules! flatten_type_list {
  ( ( $ty1:ty , ( $ty2:ty , $ty3:ty ) ) ) => {
    ( $ty1 , $ty2 , flatten_tuple! ( $ty3 ) )
  };
  ( ( $ty:ty , () ) ) => { ( $ty , ) };
  ( () ) => { () };
}
macro_rules! mk_ty {
  ($($tokens:tt)*) => {
    flatten_type_list! { mk_ty_rec! { $($tokens)* } }
  };
  () => { () };
}
#[macro_export]
macro_rules! build_capture_tuple {
  ($_:ident = $expr:expr , $( $tail:tt)*) => {
    ($expr , build_capture_tuple! { $($tail)* } )
  };
  ($id:ident , $( $tail:tt)*) => {
    ($id , build_capture_tuple! { $($tail)* } )
  };
  () => { () };
}
#[macro_export]
macro_rules! build_destructor_tuple {
  ($ident:ident = $_:expr , $( $tail:tt)*) => {
    ($ident , build_destructor_tuple! { $($tail)* } )
  };
  ($id:ident , $( $tail:tt)*) => {
    ($id , build_destructor_tuple! { $($tail)* } )
  };
  () => { () };
}
#[macro_export]
macro_rules! build_arg_destructor_tuple {
  ($id:ident , $($tail:tt)*) => {
    ($id , build_arg_destructor_tuple! { $($tail)* } )
  };
  () => { () };
}
