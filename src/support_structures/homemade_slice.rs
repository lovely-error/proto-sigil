
#[derive(Debug)]
pub struct Slice<T> {
  pub source_data: *const T,
  pub span: u32,
}

impl <T> Copy for Slice<T> where T: Copy {}

impl <T> Clone for Slice<T> where T: Clone {
  fn clone(&self) -> Self {
    Self { source_data: self.source_data, span: self.span }
  }
}