
use std::intrinsics::copy_nonoverlapping;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, size_of, align_of, forget};
use std::alloc::{alloc, Layout, dealloc};


#[derive(Debug)]
pub struct InlineVector<const stack_size : usize, Item> {
  pub stack: [MaybeUninit<Item> ; stack_size],
  pub heap: *mut Item,
  ptr: u32,
  heap_capacity: u16,
  _own_mark: PhantomData<Item>,
}

impl<const n : usize, T> InlineVector<n, T> {
  pub fn init() -> Self {
    Self {
      heap: usize::MAX as *mut T,
      ptr: 0,
      stack: unsafe { MaybeUninit::uninit().assume_init() },
      heap_capacity: 32,
      _own_mark: PhantomData
    }
  }
}

impl<const n : usize, T> InlineVector<n, T> {
  fn alloc_heap_storage(&mut self) {
    unsafe {
      let layout =
      Layout::from_size_align_unchecked(
        size_of::<T>() * self.heap_capacity as usize,
        align_of::<T>());
      let ptr = alloc(layout) as *mut T;
      self.heap = ptr;
    }
  }
  fn realloc(&mut self) {
    unsafe {
      let layout =
      Layout::from_size_align_unchecked(
        size_of::<T>() * self.heap_capacity as usize * 2,
        1);
      let fresh_mem_ptr = alloc(layout) as *mut T;
      if self.heap as usize == usize::MAX { panic!("Where's heap, huh??") }
      copy_nonoverlapping(
        self.heap, fresh_mem_ptr,
        self.heap_capacity as usize);
      dealloc(
        self.heap.cast(),
        Layout::from_size_align_unchecked(
          size_of::<T>() * self.heap_capacity as usize,
          1));
      self.heap = fresh_mem_ptr;
      self.heap_capacity *= 2;
    }
  }
  pub fn append(&mut self, new_item: T) {
    if (self.ptr as usize) < n {
      unsafe {
        *((self.stack
          .as_ptr()
          .add(self.ptr as usize)) as *mut T)
          = new_item;
      }
      self.ptr += 1;
      return ();
    }
    if self.heap as usize == usize::MAX {
      self.alloc_heap_storage();
    }
    if self.ptr > self.heap_capacity as u32 {
      self.realloc();
    }
    let index = self.ptr as usize - n;
    unsafe { *self.heap.add(index) = new_item };
    self.ptr += 1;
  }
  pub fn get_ref(&self, index: u32) -> &T {
    assert!(index < self.ptr);
    if (index as usize) < n {
      return unsafe {
        self.stack.get_unchecked(index as usize).assume_init_ref() };
    };
    return unsafe {
      &*self.heap.add(index as usize - n) };
  }
  pub fn is_empty(&self) -> bool {
    return self.ptr == 0;
  }
  pub fn count_items(&self) -> u32 {
    return self.ptr;
  }
  pub fn did_allocate_on_heap(&self) -> bool {
    return self.heap as usize != usize::MAX;
  }
  pub fn move_content_into(self, target: *mut T) { unsafe {
    copy_nonoverlapping(
      self.stack.as_ptr(), target.cast(), n);
    if self.did_allocate_on_heap() {
      copy_nonoverlapping(
        self.heap,
        target.add(n),
        self.ptr as usize - n);
    }
    forget(self);
  } }
}

//impl<const n : usize, T> Copy for InlineVector<n, T> where T:Copy {}

impl<const n : usize, T> Drop for InlineVector<n, T> {
  fn drop(&mut self) { unsafe {
    if self.did_allocate_on_heap() {
      let layout =
        Layout::from_size_align_unchecked(
          size_of::<T>() * self.heap_capacity as usize,
          1);
      dealloc(self.heap.cast(), layout);
    }
  } }
}