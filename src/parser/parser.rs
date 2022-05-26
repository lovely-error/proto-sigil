
use std::intrinsics::{transmute, copy_nonoverlapping};
use std::mem::{MaybeUninit, size_of};
use std::ptr::{addr_of_mut, addr_of};


use crate::{
  monic, monic_unfolding_loop, emit, throw, guard
};
use crate::preliminaries::monad::{
  Transient, init_with_aux_gen_ctx_intro,
  fail_with_aux_gen_ctx_intro};
use crate::parser::node_allocator::NodeSlabSizeInBytes;


pub struct ParsingState<'chars> {
  bytes: &'chars [u8],
  current_char: u32,
  pub byte_index: usize,
    // Todo: figure out how to expose fields to testing code
  line_number: u32,
  node_allocator_ptr: *mut Pager<NodeSlabSizeInBytes>,
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
  UnrecognisedCharacter, EmptySymbol, TooLongSymbol,
  UnterminatedSubexpr, UnexpectedCharacter
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
    let mut start = self.byte_index;
    loop {
      self.skip_whitespaces();
      if self.get_current_char() == '\n' {
        self.next_char();
        start = self.byte_index;
      } else { break };
    }
    return (self.byte_index - start) as u32;
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
           char == EOT || char == '=' ;
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
  RawKind, RefNode, AppNodeArgsInline, AppNodeIndirectSmall, SourceLocation, AppNodeVec, ExprPtr, PatternExprPtr, CompoundPatternNode_ArgsInline, RefPatternNode, CompoundPatternNode_ArgsIndiSlab, RewriteRule, Definition, DeclPtr, Mapping};
use crate::preliminaries::mini_vector::InlineVector;
use super::node_allocator::{Pager, EntangledPtr};


// Raw Expr parsing
impl ParsingState<'_> {
  pub fn parse_expr(
    &mut self,
    root_indentation_depth: u32
  ) -> Maybe<ExprPtr> { unsafe {
    if self.prefix_match("\\{", false) {
      let lambda = self.parse_lambda()?;
      return Ok(lambda);
    }
    let root = self.parse_symbol()?;
    let mut subexprs =
      InlineVector::<4, ExprPtr>::init();
    loop {
      self.skip_whitespaces();
      if self.prefix_match("\n", true) {
        let depth = self.probe_depth();
        if depth <= root_indentation_depth {
          self.byte_index -= depth as usize; break };
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
    let this_node_ptr =
      (*self.node_allocator_ptr).get_slot();
    if subexprs.is_empty() {
      let subexpr = RefNode {
        name: root,
        //sloc_data:
      };
      this_node_ptr.cast::<RefNode>().write(subexpr);
      let expr_ptr = ExprPtr::init(RawKind::Ref, this_node_ptr);
      return Ok(expr_ptr);
    };
    let count = subexprs.count_items();
    if count <= 4 {
      let mut data =
        MaybeUninit::<AppNodeArgsInline>::uninit();
      let ptr = data.assume_init_mut();
      ptr.name = root;
      subexprs.move_content_into(ptr.args.as_mut_ptr());
      this_node_ptr.cast::<AppNodeArgsInline>().write(data.assume_init());
      let expr_ptr = ExprPtr::init_counted_node(
        RawKind::App_ArgsInline, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };
    if count <= (NodeSlabSizeInBytes / size_of::<ExprPtr>()) as u32 {
      let invocation_data = (*self.node_allocator_ptr).get_slot();
      subexprs.move_content_into(invocation_data.cast());
      let ptr_to_arg_ptrs = EntangledPtr::from_ptr_pair(
        this_node_ptr, invocation_data)
        .expect("Values should be close enough");
      let node =
        &mut *this_node_ptr.cast::<AppNodeIndirectSmall>();
      node.name = root;
      node.args = ptr_to_arg_ptrs;
      let expr_ptr = ExprPtr::init_counted_node(
        RawKind::App_ArgsInSlab, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };

    todo!("too many args :)");
  } }
}


impl ParsingState<'_> {
  pub fn parse_lambda(&mut self) -> Maybe<ExprPtr> { unsafe {
    guard! {
      self.prefix_match("\\{", true)
        => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut depth = self.probe_depth();
    let slab = (*self.node_allocator_ptr).get_slot();
    let mut clauses =
      InlineVector::<4, EntangledPtr>::init();
    loop {
      let clause = self.parse_clause(
        depth)?;
      let ptr = EntangledPtr::from_ptr_pair(
        slab, clause.cast()).unwrap();
      clauses.append(ptr);
      depth = self.probe_depth();
      if self.prefix_match("}", true) { break; }
    }
    let count = clauses.count_items();
    if count as usize <= NodeSlabSizeInBytes / size_of::<EntangledPtr>() {
      let arg_slab = (*self.node_allocator_ptr).get_slot();
      clauses.move_content_into(arg_slab.cast());
      let node_ptr =
        ExprPtr::init_counted_node(
          RawKind::Lam, slab, count as usize);
      return Ok(node_ptr);
    }

    todo!()
  } }
  pub fn parse_clause(
    &mut self, indentation_depth: u32
  ) -> Maybe<*mut RewriteRule> { unsafe {
    guard! {
      self.prefix_match("|", true)
        => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut patterns =
      InlineVector::<4, PatternExprPtr>::init();
    loop {
      self.skip_delimiters();
      let pattern = self.parse_pattern()?;
      patterns.append(pattern);
      self.skip_delimiters();
      if self.prefix_match(",", true) {
        continue; }
      if self.prefix_match("=>", true) {
        self.skip_whitespaces(); break; }
      throw!(self.fail_with(ParseErrorKind::UnexpectedCharacter));
    }
    let depth = self.probe_depth();
    let stencil = self.parse_expr(
      if depth == 0 { indentation_depth }
                              else { depth } )?;

    let count = patterns.count_items();
    let pats = (*self.node_allocator_ptr).get_slot();
    patterns.move_content_into(pats.cast());

    if count as usize <= NodeSlabSizeInBytes / size_of::<PatternExprPtr>() {
      let slab = (*self.node_allocator_ptr).get_slot();
      let data = &mut *slab.cast::<RewriteRule>();
      data.pattern_count = count as u16;
      data.patterns = EntangledPtr::from_ptr_pair(
        slab, pats).unwrap();
      data.stencil = stencil;
      return Ok(slab.cast());
    }
    todo!()
  } }
  pub fn parse_pattern(&mut self) -> Maybe<PatternExprPtr> { unsafe {
    if self.prefix_match("_", true) {
      let wk = PatternExprPtr::init_wildcard();
      return Ok(wk);
    }
    let root = self.parse_symbol()?;
    let mut args =
      InlineVector::<4, PatternExprPtr>::init();
    loop {
      self.skip_delimiters();
      if self.at_terminator() { break; }
      if self.prefix_match("(", true) {
        let sub_pat = self.parse_pattern()?;
        args.append(sub_pat);
        self.skip_delimiters();
        guard! {
          self.prefix_match(")", true)
          => self.fail_with(ParseErrorKind::UnexpectedCharacter)
        }
        continue;
      }
      let terminal_pat = self.parse_symbol()?;
      let slab = (*self.node_allocator_ptr).get_slot();
      let node = &mut *slab.cast::<RefPatternNode>();
      node.name = terminal_pat;
      let expr = PatternExprPtr::init_singular(slab);
      args.append(expr);
    }
    let slab = (*self.node_allocator_ptr).get_slot();
    if args.is_empty() {
      let data =
        &mut *slab.cast::<RefPatternNode>();
      data.name = root;
      let var = PatternExprPtr::init_singular(slab);
      return Ok(var);
    }
    let count = args.count_items();
    if count <= 4 {
      let data =
        &mut *slab.cast::<CompoundPatternNode_ArgsInline>();
      data.name = root;
      args.move_content_into(data.args.as_mut_ptr());
      let comp =
        PatternExprPtr::init_compound_inlined(
          slab, count as u8);
      return Ok(comp);
    }
    if count <= (NodeSlabSizeInBytes / size_of::<PatternExprPtr>()) as u32 {
      let indi_args = (*self.node_allocator_ptr).get_slot();
      args.move_content_into(indi_args.cast());
      let data =
        &mut *slab.cast::<CompoundPatternNode_ArgsIndiSlab>();
      data.name = root;
      data.args =
        EntangledPtr::from_ptr_pair(
          slab, indi_args).unwrap();
      let pat_node_ptr = PatternExprPtr::init_compound_indirect(
        slab, count as u8);
      return Ok(pat_node_ptr);
    }
    todo!()
  } }
}


impl ParsingState<'_> {
  pub fn parse_decl(&mut self) -> Maybe<DeclPtr> {
    let name = self.parse_symbol()?;
    self.skip_delimiters();
    guard! {
      self.prefix_match(":", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    }
    let depth = self.probe_depth();
    let type_ =
      self.parse_expr(depth)?;
    let mut depth = self.probe_depth();
    if self.prefix_match("=", true) {
      depth = self.probe_depth();
      let value = self.parse_expr(depth)?;
      unsafe {
        let slab = (*self.node_allocator_ptr).get_slot();
        let data =
          &mut *slab.cast::<Definition>();
        data.name = name;
        data.type_ = type_;
        data.value = value;
        let def_ptr = DeclPtr::init_def(slab);
        return Ok(def_ptr);
      }
    }
    if self.prefix_match("|", false) {
      let mut clauses =
        InlineVector::<4, *mut RewriteRule>::init();
      loop {
        let clause =
          self.parse_clause(depth)?;
        clauses.append(clause);
        depth = self.probe_depth();
        if self.prefix_match("|", false) { continue; }
        else { self.byte_index -= depth as usize; break; }
      }
      let count = clauses.count_items();
      if count as usize <=
      NodeSlabSizeInBytes / size_of::<EntangledPtr>() { unsafe {
        let slab = (*self.node_allocator_ptr).get_slot();
        let slab_ = slab.cast::<EntangledPtr>();
        for i in 0 .. count {
          let rr_ptr = clauses.get_ref(i);
          let entp =
            EntangledPtr::from_ptr_pair(
              slab, (*rr_ptr).cast()).unwrap();
          *slab_.add(i as usize) = entp;
        }
        let map_decl_ptr = (*self.node_allocator_ptr).get_slot();
        let data = &mut *map_decl_ptr.cast::<Mapping>();
        data.name = name;
        data.type_ = type_;
        let rel_ptr =
          EntangledPtr::from_ptr_pair(
            map_decl_ptr, slab.cast()).unwrap();
        data.clauses = rel_ptr;
        let map_decl = DeclPtr::init_map(
          map_decl_ptr, count as u8);
        return Ok(map_decl);
      } }
      todo!()
    }
    throw!(self.fail_with(ParseErrorKind::UnexpectedCharacter));
  }
  pub fn run_parsing(&mut self) -> Maybe<Vec<()>> {
    todo!()
  }
}