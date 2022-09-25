
use std::{marker::PhantomData, mem::{size_of, transmute},};

#[derive(Debug)]
pub struct TaggedPtr<Tag: Copy, PointedValue>
  (u64, PhantomData<(Tag, PointedValue)>);

#[repr(C)]
union Repr<T:Copy> { bits: u64, value: T }

impl <T: Copy, P> TaggedPtr<T, P> {
  fn common_init(tag: T, ptr_bits: u64) -> Self {
    let tag_size = size_of::<T>();
    if tag_size > 3 {
      panic!("Tag size is too big. It cannot exceed 3 bytes. ({})", tag_size)
    }

    let tag = tag;
    let mut ptr_bits = ptr_bits << (8 * tag_size);

    let mut val = Repr { bits: 0 };
    val.value = tag ;
    unsafe { ptr_bits += val.bits } ;

    return Self(ptr_bits, PhantomData)
  }
  pub fn init_null() -> Self {
    Self(0, PhantomData)
  }
  pub fn is_null(&self) -> bool {
    self.0 == 0
  }
  pub fn init_from_ptr(tag: T, ptr: *mut P) -> Self {
    TaggedPtr::common_init(tag, ptr as u64)
  }
  pub fn project_ptr(&self) -> *mut P {
    let tag_size = size_of::<T>();
    let ptr_bits = self.0 >> (8 * tag_size);
    let ptr = ptr_bits as *mut P;

    return ptr
  }
  pub fn project_tag(&self) -> T {
    let tag_bits = Repr { bits: self.0 };
    let tag = unsafe { tag_bits.value };
    return tag
  }
  pub fn inject_ptr(&mut self, ptr: *mut P) {
    let tag_size = size_of::<T>();
    let tag_mask = (1u64 << (tag_size * 8)) - 1;
    let mut val = self.0 & tag_mask;
    val += (ptr as u64) << (tag_size * 8);
    self.0 = val;
  }
  pub fn inject_tag(&mut self, tag: T) {
    let mut val = Repr { bits: self.0 };
    val.value = tag ;
    self.0 = unsafe { val.bits };
  }
  pub fn cast<K>(&self) -> TaggedPtr<T, K> {
    unsafe { transmute(*self) }
  }
}

impl <T: Copy, V: Clone> TaggedPtr<T, V> {
  pub unsafe fn deref(&self) -> V {
    (&*self.project_ptr()).clone()
  }
}

impl <T: Copy, V> Copy for TaggedPtr<T, V> {}
impl <T: Copy, V> Clone for TaggedPtr<T, V> {
  fn clone(&self) -> Self { *self }
}