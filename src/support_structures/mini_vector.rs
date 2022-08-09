
use std::intrinsics::copy_nonoverlapping;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, size_of, align_of, needs_drop};
use std::alloc::{alloc, Layout, dealloc};
use std::ptr::{drop_in_place,};

use crate::elaborator::worker::LoopQueue;


#[derive(Debug)]
pub struct InlineVector<const stack_size : usize, Item> {
  pub(crate) stack: [MaybeUninit<Item> ; stack_size],
  pub(crate) heap: *mut Item,
  ptr: u32,
  heap_capacity: u32,
  _own_mark: PhantomData<Item>,
}

impl <const n : usize, T> InlineVector<n, T> {
  pub fn init() -> Self {
    Self {
      heap: usize::MAX as *mut T,
      ptr: 0,
      stack: unsafe { MaybeUninit::uninit().assume_init() },
      heap_capacity: (n * 2) as u32,
      _own_mark: PhantomData
    }
  }
}

impl <const n : usize, T> InlineVector<n, T> {
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
      let old_capacity = self.heap_capacity;
      self.heap_capacity *= 2;
      let layout =
      Layout::from_size_align_unchecked(
        size_of::<T>() * self.heap_capacity as usize,
        1);
      let fresh_mem_ptr = alloc(layout) as *mut T;
      if self.heap as usize == usize::MAX { panic!("Where's heap, huh??") }
      copy_nonoverlapping(
        self.heap, fresh_mem_ptr,
        old_capacity as usize);
      dealloc(
        self.heap.cast(),
        Layout::from_size_align_unchecked(
          size_of::<T>() * old_capacity as usize,
          1));
      self.heap = fresh_mem_ptr;
    }
  }
  pub fn append(&mut self, new_item: T) {
    if (self.ptr as usize) < n {
      unsafe {
        self.stack
          .as_mut_ptr()
          .add(self.ptr as usize)
          .cast::<T>()
          .write(new_item);
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
  pub fn get_ref(&self, index: usize) -> &T {
    assert!(index < self.ptr as usize);
    if (index) < n {
      return unsafe {
        self.stack.get_unchecked(index).assume_init_ref() };
    };
    return unsafe {
      &*self.heap.add(index as usize - n) };
  }
  pub fn is_empty(&self) -> bool {
    return self.ptr == 0;
  }
  pub fn count_items(&self) -> u32 {
    return self.ptr
  }
  pub fn did_allocate_on_heap(&self) -> bool {
    return self.heap as usize != usize::MAX;
  }
  pub fn move_content_into(&mut self, recepient: *mut T) { unsafe {
    copy_nonoverlapping(
      self.stack.as_ptr(), recepient.cast(),
      if self.ptr as usize <= n { self.ptr as usize } else { n });
    if self.did_allocate_on_heap() {
      copy_nonoverlapping(
        self.heap,
        recepient.add(n),
        self.ptr as usize - n);
    }
    self.ptr = 0;
  } }
  pub fn reset(&mut self) {
    if needs_drop::<T>() { todo!() }
    self.ptr = 0;
  }
  pub fn pop(&mut self) -> Option<T> { unsafe {
    if self.ptr == 0 { return None };
    self.ptr -= 1;
    let ptr = self.ptr as usize;
    if ptr >= n {
      let item = self.heap.add(ptr - n).read();
      return Some(item);
    };
    let item =
      self.stack.as_mut_ptr().add(ptr).cast::<T>().read();
    return Some(item)
  } }
}

impl <const n : usize, T> Drop for InlineVector<n, T> {
  fn drop(&mut self) { unsafe {
    let should_run_destructor = needs_drop::<T>();
    if should_run_destructor {
      for i
      in 0 .. if self.ptr as usize <= n { self.ptr as usize } else { n } {
        let ptr =
          self.stack
          .as_mut_ptr()
          .add(i)
          .cast::<T>();
        drop_in_place(ptr);
      }
    }
    if self.did_allocate_on_heap() {
      if should_run_destructor {
        for i in 0 .. self.ptr as usize - n {
          let ptr = self.heap.add(i).cast::<T>();
          drop_in_place(ptr);
        }
      }
      let layout =
        Layout::from_size_align_unchecked(
          size_of::<T>() * self.heap_capacity as usize,
          1);
      dealloc(self.heap.cast(), layout);
    }
  } }
}

impl <const n : usize, T> InlineVector<n, T> where T: Copy {
  fn copy_content_into(&self, recepient: *mut T) { unsafe {
    copy_nonoverlapping(
      self.stack.as_ptr(), recepient.cast(),
      if self.ptr as usize <= n { self.ptr as usize } else { n });
    if self.did_allocate_on_heap() {
      copy_nonoverlapping(
        self.heap,
        recepient.add(n),
        self.ptr as usize - n);
    }
  } }
}

impl <const n : usize, T> InlineVector<n, T> where T: Clone {
  fn clone_content_into(&self, recepient: *mut T) {
    for i
    in 0 .. if  self.ptr as usize <= n { self.ptr as usize } else { n } {
      let cloned_item = self.get_ref(i).clone();
      unsafe { recepient.add(i as usize).write(cloned_item) };
    }
    if self.did_allocate_on_heap() {
      for i in 0 .. (self.ptr as usize - n) { unsafe {
        let cloned_item =
          self.heap.add(i).as_ref().unwrap().clone();
        recepient.add(i + n).write(cloned_item);
      } }
    }
  }
}

impl <const n : usize, T> InlineVector<n, T> where T:Copy {
  // not too fancy impl, tbh.
  // but making it perform faster is harder
  pub fn copy_quickly_into(&self, target: &mut LoopQueue<T>) { unsafe {
    if self.is_empty() { return; }
    let end_index = self.count_items() as usize;
    let dst_capacity =
      (4096 / size_of::<T>()) - target.write_ptr as usize;
    if dst_capacity > end_index {
      // do it in one take
      let dst =
        target.write_page.add(target.write_ptr as usize);
      self.copy_content_into(dst);
      target.write_ptr += end_index as u16;
      return;
    }
    // do it bit by bit
    for i in 0 .. end_index {
      let item = self.get_ref(i);
      target.enqueue_item(*item);
    }
  } }
}

pub trait SomeInlineVector {
  type Item;
  fn push(&mut self, item: Self::Item);
}

impl <const n : usize, T> SomeInlineVector for InlineVector<n, T> {
  type Item = T;
  fn push(&mut self, item: Self::Item) {
    self.append(item)
  }
}