
use std::{alloc::{Layout, alloc, dealloc}, mem::size_of};


const Page4K : Layout = unsafe {
  Layout::from_size_align_unchecked(4096, 1) } ;

pub const NodeSlabSizeInBytes : usize = 64;

const NodePageCapacity : u16 = 4096 / NodeSlabSizeInBytes as u16 ;


#[derive(Debug, Copy, Clone)]
pub struct EntangledPtr(pub i32);
impl EntangledPtr {
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

#[derive(Debug)]
pub struct SlabAllocator<const item_size : usize> {
  pub first_page: *mut (),
  pub last_page: *mut (),
  pub current_page: *mut (),
  pub ptr: u16,
  pub capacity: u32
}

impl <const s : usize> SlabAllocator<s> {
  pub fn init() -> Self { unsafe {
    let page = alloc(Page4K).cast::<()>();
    *page.cast::<usize>() = usize::MAX;
    return Self { first_page: page,
                  last_page: page,
                  current_page: page,
                  ptr: (size_of::<usize>() / s).max(1) as u16,
                  capacity: 4096 }
  } }
}

impl <const s : usize> SlabAllocator<s> {
  pub fn get_slot(&mut self) -> *mut () { unsafe {
    let product =
      self.current_page
      .cast::<[u8;s]>()
      .add(self.ptr as usize)
      .cast::<()>();
    self.ptr += 1;
    if self.ptr == 4096 / s as u16 {
      let fresh_page = alloc(Page4K);
      *fresh_page.cast::<*mut ()>() = self.last_page;
      self.last_page = self.current_page;
      self.current_page = fresh_page.cast::<()>();
      self.capacity += 4096;
    }
    return product;
  } }
}

impl <const s : usize> Drop for SlabAllocator<s> {
  fn drop(&mut self) { unsafe {
    dealloc(self.current_page.cast(), Page4K);
    if self.current_page == self.last_page { return (); }
    let mut ptr = self.last_page.cast::<u8>();
    loop {
      let tail = *ptr.cast::<usize>();
      dealloc(ptr, Page4K);
      if tail == usize::MAX { break; }
      ptr = tail as *mut _ ;
    }
  } }
}

