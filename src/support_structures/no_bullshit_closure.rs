
use std::{
  mem::{size_of, align_of, }, ptr::{null_mut, drop_in_place,},
  alloc::{alloc, Layout, dealloc}, marker::PhantomData,
  intrinsics::{transmute,},};

use crate::parser::node_allocator::EntangledPtr;


pub trait SupportTypeErasure<Erased> {
  fn erase_type(self) -> Erased;
}


#[repr(align(16))]
struct NoDctorClos {
  env: *mut (),
  inv: *mut (),
}

#[repr(align(16))]
pub struct Closure<Env, I, O> {
  env: *mut (),
  inv: EntangledPtr,
  dctor: EntangledPtr,

  _own1: PhantomData<I>,
  _own2: PhantomData<O>,
  _own3: PhantomData<Env>
}

impl <X, Y, I> Closure<X, Y, I> {
  fn dctor(env: *mut ()) { unsafe {
    if env != null_mut() {
      // need to track if calling happened
      // drop_in_place(env.cast::<X>());
      dealloc(
        env.cast::<u8>(),
        Layout::from_size_align_unchecked(
          size_of::<X>(), align_of::<X>()))
    }
  } }
  pub fn init_with_given_mem(
    mem: *mut (), env: X, fun: fn (*mut X, Y) -> I
  ) -> Self { unsafe {
    mem.cast::<X>().write(env);
    let fun =
      EntangledPtr::from_ptr_pair(mem, fun as *mut ())
      .unwrap();
    let dctor =
      EntangledPtr::from_ptr_pair(
        mem, Self::dctor as *mut ())
      .unwrap();
    return Self {
      env: mem, inv: fun, dctor,
      _own1: PhantomData, _own2: PhantomData, _own3: PhantomData
    }
  } }
  pub fn init_with_global_mem(
    env: X, fun: fn (*mut X, Y) -> I
  ) -> Self { unsafe {
    if size_of::<X>() != 0 {
      let mem = alloc(Layout::from_size_align_unchecked(
        size_of::<X>(), align_of::<X>()));
      mem.cast::<X>().write(env);
      let fun =
        EntangledPtr::from_ptr_pair(
          mem.cast(), fun as *mut ())
        .unwrap();
      let dctor =
        EntangledPtr::from_ptr_pair(
          mem.cast(), Self::dctor as *mut ())
        .unwrap();
      return Self {
        env: mem.cast(), inv: fun, dctor,
        _own1: PhantomData, _own2: PhantomData, _own3: PhantomData
      };
    } else {
      let triv_clos =
        NoDctorClos { env: null_mut(), inv: fun as *mut () };
      return transmute(triv_clos)
    };
  }; }
  pub fn invoke_dispose(self, args: Y) -> I { unsafe {
    if self.env == null_mut() { // trivial closure
      let triv_clos =
        transmute::<_, NoDctorClos>(self);
      let fun =
        transmute::<_, fn (u64, Y) -> I>(triv_clos.inv);
      let res = fun (0, args);
      return res;
    } else {
      let stan_clos = self;
      let fun =
        stan_clos.inv.reach_referent_from(stan_clos.env)
        .cast::<fn (*mut X, Y) -> I>();
      let res = (*fun)(&mut *stan_clos.env.cast::<X>(), args);
      drop(stan_clos);
      return res;
    }
  } }
}

impl <X, Y, I> Drop for Closure<X, Y, I> {
  fn drop(&mut self) { unsafe {
    if self.env != null_mut() {
      let dctor_ptr =
        self.dctor.reach_referent_from(self.env);
      let dctor = transmute::<_, fn (*mut ())>(dctor_ptr);
      dctor (self.env);
    }
  } }
}

#[repr(align(16))]
pub struct SomeClosure<I, O> {
  env: *mut (),
  inv: EntangledPtr,
  dctor: EntangledPtr,

  _own1: PhantomData<I>,
  _own2: PhantomData<O>,
}

#[repr(align(16))]
pub struct SomeTrivialClosure<I, O> {
  env: *mut (),
  inv: *mut (),

  _own1: PhantomData<I>,
  _own2: PhantomData<O>,
}

impl <I, O> SomeClosure<I, O> {
  pub fn invoke_consume(self, args: I) -> O { unsafe {
    if self.env == null_mut() {
      let triv_clos =
        transmute::<_, SomeTrivialClosure<I, O>>(self);
      let fun =
        transmute::<_, fn (*mut (), I) -> O>(triv_clos.inv);
      let res = fun (null_mut(), args);
      return res;
    } else {
      let fun_ptr = self.inv.reach_referent_from(self.env);
      let fun =
        transmute::<_, fn (*mut (), I) -> O>(fun_ptr);
      let res = fun (self.env, args);
      drop(self);
      return res;
    }
  } }
}

impl <I, O> Drop for SomeClosure<I, O> {
  fn drop(&mut self) { unsafe {
    if self.env != null_mut() {
      let ptr = self.dctor.reach_referent_from(self.env);
      let dctor = transmute::<_, fn (*mut ())>(ptr);
      dctor (self.env);
    }
  } }
}

impl <X, Y, I> Clone for Closure<X, Y, I> where X:Clone {
  fn clone(&self) -> Self { unsafe {
    if self.env == null_mut() { // just copy bytes
      let copy =
        transmute::<_, *const Self>(self).read();
      return copy;
    } else { // need memcopy env
      let new_env_mem =
        alloc(Layout::from_size_align_unchecked(
          size_of::<X>(), align_of::<X>()));
      let copy = (&*self.env.cast::<X>()).clone();
      new_env_mem.cast::<X>().write(copy);
      return Self {
        dctor: self.dctor, env: new_env_mem.cast(), inv: self.inv,
        _own1: PhantomData, _own2: PhantomData, _own3: PhantomData
      };
    }
  } }
}

unsafe impl <X, Y, I> Send for Closure<X, Y, I> where X:Send {}
unsafe impl <X, Y, I> Sync for Closure<X, Y, I> where X:Sync {}


impl <X, Y, I> SupportTypeErasure<SomeClosure<Y, I>> for Closure<X, Y, I> {
  fn erase_type(self) -> SomeClosure<Y, I> {
    unsafe { transmute(self) }
  }
}


#[macro_export]
macro_rules! closure {
  ( [ $($capt_name:ident),* ]
    $(| $( $arg_name:ident $(: $ty:ty)? ),* |)?
    $(-> $rt:ty)? $bl:block )
  => {
    {
      let env = ($($capt_name),*);
      // fn name_env_type<X, Y, I>() -> fn (*mut X, Y) -> I {
      //   | env : *mut X , Y | -> I {
      //     let ($($capt_name ),*) = unsafe { env.cast::<X>().read() };
      //     $bl
      //   }
      // }
      let clos = Closure::init_with_global_mem(
        env, | env , ( $($($arg_name),*)? )
          // : mk_ty! { $($arg_name $(: $ty)? ,)* }
        |
        $(-> $rt)? {
          //name_env_type()(env, ( $($arg_name),* ))
          let ($($capt_name ),*) = unsafe { env.read() };
          $bl
        });
      let some_clos = clos.erase_type();
      some_clos
    }
  };
}
macro_rules! mk_ty_rec {
  ($_:ident : $ty:ty , $($tail:tt)*) => {
    ( $ty , mk_ty_rec! { $($tail)* } )
  };
  ($_:ident , $($tail:tt)*) => {
    ( _ , mk_ty_rec! { $($tail)* } )
  };
  () => { () };
}
macro_rules! flatten_tuple {
  ( ( $ty1:ty , ( $ty2:ty , $ty3:ty ) ) ) => {
    ( $ty1 , $ty2 , flatten_tuple! ( $ty3 ) )
  };
  ( ( $ty:ty , () ) ) => { ( $ty , ) };
  ( () ) => { () };
}
macro_rules! mk_ty {
  ($($tokens:tt)*) => {
    flatten_tuple! { mk_ty_rec! { $($tokens)* } }
  };
  () => { () };
}


fn test () {
  let str = "!".to_string();
  let _  = closure!([str] | gavno : String , mocha | {
    let _ : String = gavno;
    let _ : usize = mocha ;
    println!("{}", str)
  });
}