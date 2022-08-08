
use std::{alloc::{Layout, alloc, dealloc}, mem::size_of, panic, marker::PhantomData};


const Page4K : Layout = unsafe {
  Layout::from_size_align_unchecked(4096, 1) } ;


#[derive(Debug, Copy, Clone)]
pub struct SomeEntangledPtr(pub i32);
impl SomeEntangledPtr {
  pub fn init_null() -> Self {
    Self(0)
  }
  pub fn is_null(&self) -> bool {
    self.0 == 0
  }
  pub fn from_ptr_pair(
    origin: *mut (),
    referent: *mut ()
  ) -> Option<Self> { unsafe {
    let diff =
      referent.cast::<u8>()
              .offset_from(origin.cast());
    if diff.abs() > (u32::MAX >> 1) as isize {
      return None;
    };
    return Some(Self(diff as i32));
  } }
  pub fn reach_referent_from(&self, origin: *mut ()) -> *mut () { unsafe {
    origin.cast::<u8>().offset(self.0 as isize).cast()
  } }
}

pub struct EntagledPtr<T>(i32, PhantomData<T>);
impl <T> EntagledPtr<T> {
  pub fn init_null() -> Self {
    Self(0, PhantomData)
  }
  pub fn is_null(&self) -> bool {
    self.0 == 0
  }
  pub fn from_ptr_pair(
    origin: *mut (),
    referent: *mut T
  ) -> Option<Self> { unsafe {
    let diff =
      referent.cast::<u8>()
              .offset_from(origin.cast());
    if diff.abs() > (u32::MAX >> 1) as isize {
      return None;
    };
    return Some(Self(diff as i32, PhantomData));
  } }
  pub fn reach_referent_from(&self, origin: *mut ()) -> *mut T { unsafe {
    origin.cast::<u8>().offset(self.0 as isize).cast()
  } }
}

#[derive(Debug)]
pub struct LinearAllocator<const MIN_ALLOC_SIZE : usize> {
  pub first_page: *mut (),
  pub current_page: *mut (),
  pub ptr: u16,
}

impl <const s : usize> LinearAllocator<s> {
  pub fn init() -> Self { unsafe {
    let page = alloc(Page4K).cast::<()>();
    *page.cast::<usize>() = usize::MAX;
    return Self { first_page: page,
                  current_page: page,
                  ptr: (size_of::<usize>() / s).max(1) as u16, }
  } }
}

impl <const s : usize> LinearAllocator<s> {
  fn expand_storage(&mut self) { unsafe {
    let fresh_page = alloc(Page4K);
    *fresh_page.cast::<usize>() = usize::MAX;
    *self.current_page.cast::<*mut u8>() = fresh_page;
    self.current_page = fresh_page.cast::<()>();
    self.ptr = (size_of::<usize>() / s).max(1) as u16;
  } }
  pub fn get_slot(&mut self) -> *mut () { unsafe {
    let product =
      self.current_page
      .cast::<[u8;s]>()
      .add(self.ptr as usize)
      .cast::<()>();
    self.ptr += 1;
    if self.ptr == 4096 / s as u16 {
      self.expand_storage();
    }
    return product;
  } }
  pub fn get_contiguos_mem_for<T>(&mut self) -> *mut T {
    let size_ = size_of::<T>() ;
    let mem = self.get_contiguos_mem(size_);

    return mem.cast::<T>()
  }
  pub fn get_contiguos_mem(&mut self, byte_count: usize) -> *mut () {
    let size_ = byte_count;
    if size_ >= 4096 - 8 {
      panic!("Too much memory has been requested!")
    }
    let mut size = size_ / s;
    if (size_ - (size * s)) != 0 { size += 1 }

    let cap = (4096 / s) - self.ptr as usize;
    if size >= cap {
      self.expand_storage()
    }
    let mem = unsafe { self.current_page
      .cast::<[u8;s]>()
      .add(self.ptr as usize)
      .cast::<()>() };
    self.ptr += size as u16;

    return mem
  }
}

impl <const s : usize> Drop for LinearAllocator<s> {
  fn drop(&mut self) { unsafe {
    let mut ptr = self.first_page.cast::<u8>();
    loop {
      let tail = *ptr.cast::<usize>();
      dealloc(ptr, Page4K);
      if tail == usize::MAX { break; }
      ptr = tail as *mut _ ;
    }
  } }
}

