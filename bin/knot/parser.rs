
pub struct CLIParseState {
  pub bytes: *const u8,
  current_char: u32,
  pub byte_index: u32,
  end_index: u32,
}

const EOT : char = '\u{3}' ;

pub struct Checkpoint {
  old_char: u32,
  old_ptr: u32,
}

impl CLIParseState {
  fn no_more_chars(&self) -> bool {
    self.byte_index == self.end_index
  }
  fn next_char(&mut self) {
    if self.no_more_chars() { return (); }
    // only ascii subset for now
    self.byte_index += 1;
    self.current_char = unsafe {
      *self.bytes.add(self.byte_index as usize) as u32 };
  }
  pub fn get_current_char(&self) -> char {
    if self.no_more_chars(){ return EOT; }
    return unsafe {
      char::from_u32_unchecked(self.current_char) };
  }
  pub fn skip_while(
    &mut self,
    mut predicate: impl FnMut(&mut Self) -> bool
  ) {
    loop {
      if self.no_more_chars() { break; }
      if !predicate(self) { break; }
      self.next_char();
    }
  }
  pub fn skip_whitespaces(&mut self) {
    self.skip_while(|self_| {
      let char = self_.get_current_char();
      return char == ' ';
    })
  }
  fn make_position_snapshot(&self) -> Checkpoint {
    Checkpoint {
      old_char: self.current_char,
      old_ptr: self.byte_index,
    }
  }
  pub fn backtrack_state_to(
    &mut self,
    Checkpoint { old_char, old_ptr }: Checkpoint
  ) {
    self.byte_index = old_ptr;
    self.current_char = old_char;
  }
  pub fn prefix_match(&mut self, pattern: &str, should_strip: bool) -> bool {
    let chkpt = self.make_position_snapshot();
    let mut iter = pattern.chars();
    loop {
      let item = iter.next();
      match item {
        Some(char) => {
          if self.get_current_char() != char {
            self.backtrack_state_to(chkpt);
            return false;
          }
          self.next_char();
        },
        None => break
      }
    }
    if !should_strip {
      self.backtrack_state_to(chkpt);
    }
    return true;
  }
}

impl CLIParseState {
  pub fn init(command_ptr: *const u8, length: u32) -> Self { unsafe {
    let first_char = *command_ptr as u32;
    return Self { bytes: command_ptr,
                  current_char: first_char,
                  byte_index: 0,
                  end_index: length }
  } }
  pub fn set_new_command(&mut self, length: u32) {
    self.byte_index = 0;
    self.current_char = unsafe { *self.bytes as u32 };
    self.end_index = length;
  }
}