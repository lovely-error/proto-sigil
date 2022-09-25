use crate::expression_trees::better_nodes::ArrayPtr;


pub struct RawArrayIter<T> {
  array: *mut T,
  index: u32,
  length: u32,
}

impl <T> RawArrayIter<T> {
  pub fn new(array: *mut T, length: u32) -> Self {
    return Self {
      array,
      index: 0,
      length,
    }
  }
  pub fn from_array_ptr(val: ArrayPtr<T>) -> Self {
    let ptr = val.0.project_ptr();
    let length = val.0.project_tag().length as u32;
    return Self::new(ptr, length)
  }
}

impl <T: Clone> Iterator for RawArrayIter<T> {
  type Item = T;
  fn next(&mut self) -> Option<Self::Item> {
    if self.index < self.length {
      let item = unsafe { (&*self.array.add(self.index as usize)).clone() };
      self.index += 1;
      return Some(item);
    }
    return None;
  }
}

