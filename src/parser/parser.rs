
use std::intrinsics::{transmute, copy_nonoverlapping};
use std::mem::{MaybeUninit, size_of};
use std::ptr::{addr_of_mut, addr_of};

use crate::{
  monic, monic_unfolding_loop, emit, throw, guard
};
use crate::preliminaries::monad::{
  Transient, init_with_aux_gen_ctx_intro,
  fail_with_aux_gen_ctx_intro};
use crate::parser::node_allocator::NodeSizeInBytes;


pub struct ParsingState<'chars> {
  bytes: &'chars [u8],
  current_char: u32,
  pub byte_index: usize,
    // Todo: figure out how to expose fields to testing code
  line_number: u32,
  node_allocator_ptr: *mut Pager<NodeSizeInBytes>,
  total_char_count: u32,
}

fn is_valid_char_for_symbol(char: u32) -> bool {

  if // Basic Latin small letterr
     0x61 <= char && char <= 0x7A ||
     // Basic Latin Capital Letters
     0x41 <= char && char <= 0x5A ||
     // _ symbol
     char == 0x5F { return true; }
  return false;
}

const EOT : char = '\u{3}' ;

pub struct Checkpoint {
  old_char: u32,
  old_ptr: usize,
  old_ln_num: u32
}

pub mod symbol {
  #[derive(Debug, Copy, Clone)]
  pub enum Repr {
    Inlined([u8 ; 7]),
    // [offset_from_start ; offset_from_head)
    OffsetInfo {
      offset_from_start: u32,
      offset_from_head: u16
    }
  }
  #[derive(Debug, Copy, Clone)]
  pub struct Symbol {
    pub repr: Repr
  }
}


#[derive(Debug)]
pub enum ParseErrorKind {
  UnrecognisedCharacter(char), EmptySymbol, TooLongSymbol,
  UnterminatedSubexpr
}
#[derive(Debug)]
pub struct ParseError {
  kind: ParseErrorKind,
  absolute_offset: usize
}

type Maybe<T> = Result<T, ParseError>;


/// Initializers
impl<'v> ParsingState<'v> {
  pub fn init(char_ptr: &'v [u8], allocator: *mut Pager<64>) -> Self {
    Self {
      byte_index: 0,
      bytes: char_ptr,
      current_char: unsafe { *char_ptr.get_unchecked(0) as u32 },
      line_number: 1,
      node_allocator_ptr: allocator,
      total_char_count: 0
    }
  }
}

/// Preliminary methods
impl ParsingState<'_> {
  fn fail_with(&self, error: ParseErrorKind) -> ParseError {
    ParseError { absolute_offset: self.byte_index, kind: error }
  }
  fn no_more_chars(&self) -> bool {
    self.byte_index == self.bytes.len()
  }
  fn next_char(&mut self) {
    if self.no_more_chars() { return (); }
    // only ascii subset for now
    let char = self.get_current_char();
    if char == '\n' { self.line_number += 1; };
    self.byte_index += 1;
    self.total_char_count += 1;
    self.current_char = unsafe {
      *self.bytes.get_unchecked(self.byte_index) as u32 };
  }
  fn get_current_char(&self) -> char {
    if self.no_more_chars(){ return EOT; }
    return unsafe {
      char::from_u32_unchecked(self.current_char) };
  }
  fn make_state_snapshot(&self) -> Checkpoint {
    Checkpoint {
      old_char: self.current_char,
      old_ptr: self.byte_index,
      old_ln_num: self.line_number
    }
  }
  pub fn backtrack_state_to(
    &mut self,
    Checkpoint { old_char, old_ptr, old_ln_num }: Checkpoint
  ) {
    self.byte_index = old_ptr;
    self.current_char = old_char;
    self.line_number = old_ln_num;
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
  pub fn skip_delimiters(&mut self) {
    self.skip_while(|self_| {
      let char = self_.get_current_char();
      return char == '\n' || char == ' ';
    });
  }
  pub fn skip_whitespaces(&mut self) {
    self.skip_while(|self_| {
      let char = self_.get_current_char();
      return char == ' ';
    })
  }
  pub fn probe_depth(&mut self) -> u32 {
    let mut start = self.total_char_count;
    loop {
      self.skip_whitespaces();
      if self.get_current_char() == '\n' {
        self.next_char();
        start = self.total_char_count;
      } else { break };
    }
    return self.total_char_count - start;
  }
  pub fn prefix_match(&mut self, pattern: &str, should_strip: bool) -> bool {
    let chkpt = self.make_state_snapshot();
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
  pub fn at_terminator(&self) -> bool {
    let char = self.get_current_char();
    return char == ':' || char == ')' ||
           char == '|' || char == ',' ||
           char == ']' || char == '}' ||
           char == EOT ;
  }
}

/// Symbol parsing
impl ParsingState<'_> {
  pub fn parse_symbol(&mut self) -> Maybe<symbol::Symbol> {
    use self::symbol::{Symbol, Repr};
    let symbol_start = self.byte_index;
    self.skip_while(|self_|{
      let char = self_.current_char;
      return is_valid_char_for_symbol(char);
    });
    let symbol_end = self.byte_index;
    let diff = symbol_end - symbol_start;
    if diff == 0 {
      throw!(self.fail_with(ParseErrorKind::EmptySymbol));
    };
    if diff <= 7 {
      let mut ptr = 0usize;
      let chars: [u8;7] = [0;7];
      let chars_ptr = chars.as_ptr() as usize;
      loop { unsafe {
        *((chars_ptr + ptr) as *mut u8) =
          *self.bytes.get_unchecked(symbol_start + ptr);
        ptr += 1;
        if ptr == diff { break }
      } }
      return Ok(Symbol { repr: Repr::Inlined(chars) });
    };
    if diff >= u16::MAX as usize {
      throw!(self.fail_with(ParseErrorKind::TooLongSymbol));
    };
    return Ok(Symbol { repr:
      Repr::OffsetInfo {
        offset_from_start: symbol_start as u32,
        offset_from_head: diff as u16 } });
  }
}


use crate::trees::raw_syntax_nodes::{
  RawKind, RefNode, AppNodeArgsInline, AppNodeIndirectSmall, SourceLocation, AppNodeVec, ExprPtr};
use crate::preliminaries::mini_vector::InlineVector;
use super::node_allocator::{Pager, EntangledPtr};


// Raw Expr parsing
impl ParsingState<'_> {
  pub fn parse_expr(
    &mut self,
    root_indentation_depth: u32
  ) -> Maybe<ExprPtr> { unsafe {
    let root = self.parse_symbol()?;
    let this_node_ptr =
      (*self.node_allocator_ptr).get_slot();
    let mut subexprs =
      InlineVector::<4, ExprPtr>::init();
    loop {
      self.skip_whitespaces();
      if self.prefix_match("\n", true) {
        let depth = self.probe_depth();
        if depth <= root_indentation_depth { break };
      }
      if self.at_terminator() { break; }
      if self.prefix_match("(", true) {
        self.skip_delimiters();
        let subexpr =
          self.parse_expr(root_indentation_depth)?;
        subexprs.append(subexpr);
        self.skip_delimiters();
        guard!(
          self.prefix_match(")", true)
          => self.fail_with(ParseErrorKind::UnterminatedSubexpr));
        continue;
      }
      let terminal_subexpr = self.parse_symbol()?;
      let mem_ptr = (*self.node_allocator_ptr).get_slot();
          let subexpr = RefNode {
            name: terminal_subexpr,
            //sloc_data:
          };
          mem_ptr.cast::<RefNode>().write(subexpr);
          let expr_ptr = ExprPtr::init(RawKind::Ref, mem_ptr);
          subexprs.append(expr_ptr);
    };
    if subexprs.is_empty() {
      let subexpr = RefNode {
        name: root,
        //sloc_data:
      };
      this_node_ptr.cast::<RefNode>().write(subexpr);
      let expr_ptr = ExprPtr::init(RawKind::Ref, this_node_ptr);
      return Ok(expr_ptr);
    };
    let count = subexprs.count();
    if count <= 4 {
      let mut data =
        MaybeUninit::<AppNodeArgsInline>::uninit();
      let ptr = data.assume_init_mut();
      ptr.name = root;
      subexprs.move_content_into(ptr.args.as_mut_ptr());
      this_node_ptr.cast::<AppNodeArgsInline>().write(data.assume_init());
      let expr_ptr = ExprPtr::init_app_node(
        RawKind::App_ArgsInline, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };
    if count <= (NodeSizeInBytes / size_of::<ExprPtr>()) as u32 {
      let invocation_data = (*self.node_allocator_ptr).get_slot();
      subexprs.move_content_into(invocation_data.cast());
      let ptr_to_arg_ptrs = EntangledPtr::from_ptr_pair(
        this_node_ptr, invocation_data)
        .expect("Values should be close enough");
      let node =
        &mut *this_node_ptr.cast::<AppNodeIndirectSmall>();
      node.name = root;
      node.args = ptr_to_arg_ptrs;
      let expr_ptr = ExprPtr::init_app_node(
        RawKind::App_ArgsInSlab, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };
    let mut args = Vec::new();
    args.reserve_exact(count as usize);
    subexprs.move_content_into(args.as_mut_ptr());
    let node_ptr = &mut *this_node_ptr.cast::<AppNodeVec>();
    node_ptr.name = root;
    node_ptr.args = args;
    let expr_ptr = ExprPtr::init_app_node(
      RawKind::App_ArgsInVec, this_node_ptr, count as usize);

    return Ok(expr_ptr);
  } }
}