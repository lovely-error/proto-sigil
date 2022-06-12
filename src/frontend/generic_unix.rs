
use std::path::PathBuf;

pub struct Frontend {
  target_batch: Vec<PathBuf>
}
impl Frontend {
  pub fn init() -> Self {
    Self { target_batch: Vec::new() }
  }
  pub fn attach_source_file(&mut self, file_path: PathBuf) {
    self.target_batch.push(file_path)
  }
}