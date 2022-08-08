

use std::mem::{size_of};
use std::str;
use crate::support_structures::homemade_slice::Slice;
use crate::expression_trees::better_nodes::{
  RawNodeRepr, RawNode, ArrayPtr, RawImplicitCtx, RawRewriteRule,
  RawPattern, RawPatternKind, Declaration, DeclKind, Symbol};
use crate::{ throw, guard };
use crate::support_structures::monad::{
  fail_with_aux_gen_ctx_intro };
use crate::expression_trees::raw_syntax_nodes::{RawKind, SourceLocation,};
use crate::support_structures::mini_vector::InlineVector;
use super::node_allocator::{LinearAllocator,};


#[derive(Debug, Clone, Copy)]
pub struct TempSlocInfo {
  pub primary_offset: u32
}

const MINIMUM_ALLOC_SIZE: usize = 16;

pub struct ParsingState {
  bytes: Slice<u8>,
  current_char: u32,
  pub byte_index: usize,
    // Todo: figure out how to expose fields to testing code
  line_number: u32,
  lin_alloc: Option<LinearAllocator<MINIMUM_ALLOC_SIZE>>,
}

fn is_valid_char_for_symbol(char: u32) -> bool {

  if // Basic Latin small letterr
     (0x61 <= char && char <= 0x7A) ||
     // Basic Latin Capital Letters
     (0x41 <= char && char <= 0x5A) ||
     // _ symbol
     char == 0x5F { return true; }
  return false;
}

const EOT : char = '\u{3}' ;

#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
  old_char: u32,
  old_ptr: usize,
  old_ln_num: u32
}


#[derive(Debug)]
pub enum ParseErrorKind {
  UnrecognisedCharacter, EmptySymbol, TooLongSymbol,
  UnterminatedSubexpr, UnexpectedCharacter
}
#[derive(Debug)]
pub struct ParseError {
  pub kind: ParseErrorKind,
  pub absolute_offset: usize
}

type Maybe<T> = Result<T, ParseError>;


/// Initializers
impl ParsingState {
  pub fn init(chars: &String) -> Self {
    let slice = Slice {
      source_data: chars.as_ptr(),
      span: chars.len() as u32
    };
    return Self {
      byte_index: 0,
      bytes: slice,
      current_char: unsafe { *chars.as_ptr() as u32 },
      line_number: 1,
      lin_alloc: None,

    }
  }
}

/// Accessors
impl ParsingState {
  fn allocate<T>(&mut self, object: T) -> *mut T {
    let mem = self.get_raw_mem(size_of::<T>());
    unsafe {
      let ptr = mem as *mut T;
      ptr.write(object);
      return ptr;
    }
  }
  fn get_raw_mem(&mut self, byte_count: usize) -> *mut () {
    if let None = self.lin_alloc {
      let fresh_mem_man =
        LinearAllocator::<MINIMUM_ALLOC_SIZE>::init();
      self.lin_alloc = Some(fresh_mem_man);
    }
    if let Some(ref mut mem_man) = self.lin_alloc {
      return mem_man.get_contiguos_mem(byte_count)
    }
    unreachable!()
  }
  fn get_mem<T>(&mut self, count: usize) -> *mut T {
    let count = size_of::<T>() * count;
    return self.get_raw_mem(count).cast::<T>();
  }
}

/// Preliminary methods
impl ParsingState {
  fn fail_with(&self, error: ParseErrorKind) -> ParseError {
    ParseError { absolute_offset: self.byte_index, kind: error }
  }
  pub fn no_more_chars(&self) -> bool {
    self.byte_index == self.bytes.span as usize
  }
  fn next_char(&mut self) {
    if self.no_more_chars() { return (); }
    // only ascii subset for now
    let char = self.get_current_char();
    if char == '\n' { self.line_number += 1; };
    self.byte_index += 1;
    self.current_char = unsafe {
      let char_ptr =
        self.bytes.source_data.add(self.byte_index as usize);
      *char_ptr as u32
    };
  }
  fn get_current_char(&self) -> char {
    if self.no_more_chars(){ return EOT; }
    return unsafe {
      char::from_u32_unchecked(self.current_char) };
  }
  fn checkpoint(&self) -> Checkpoint {
    Checkpoint {
      old_char: self.current_char,
      old_ptr: self.byte_index,
      old_ln_num: self.line_number
    }
  }
  pub fn backtrack_to(
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
  pub fn skip_trivia(&mut self) {
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
    let mut start = 0;
    loop {
      self.skip_whitespaces();
      if self.get_current_char() == '\n' {
        self.next_char();
        self.line_number += 1;
        start = self.byte_index;
      } else { break };
    }
    if start == 0 { return 0; }
    return (self.byte_index - start) as u32;
  }
  pub fn prefix_match(&mut self, pattern: &str, should_strip: bool) -> bool {
    let chkpt = self.checkpoint();
    let mut iter = pattern.chars();
    loop {
      let item = iter.next();
      match item {
        Some(char) => {
          if self.get_current_char() != char {
            self.backtrack_to(chkpt);
            return false;
          }
          self.next_char();
        },
        None => break
      }
    }
    if !should_strip {
      self.backtrack_to(chkpt);
    }
    return true;
  }
  pub fn at_terminator(&self) -> bool {
    let char = self.get_current_char();
    return !is_valid_char_for_symbol(char as u32);
  }
  pub fn begin_sloc(&self) -> TempSlocInfo {
    TempSlocInfo { primary_offset: self.byte_index as u32 }
  }
  pub fn end_sloc(&self, initial_loc: TempSlocInfo) -> SourceLocation {
    SourceLocation { primary_offset: initial_loc.primary_offset,
                     secondary_offset: self.byte_index as u16 }
  }
  pub fn accept_first_parse
  <const N : usize, T>(&mut self, opts: [impl FnOnce(&mut Self) -> Maybe<T> ; N])
  -> Maybe<T> {
    let chkpt = self.checkpoint();
    for i in opts.into_iter() {
      let smth = i(self);
      match smth {
        Ok(val) => return Ok(val),
        Err(_) => self.backtrack_to(chkpt),
      }
    };
    throw! {
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    }
  }
}


/// Symbol parsing
impl ParsingState {
  pub fn parse_symbol(&mut self) -> Maybe<Symbol> {
    let loc = self.begin_sloc();
    let symbol_start = self.byte_index;
    self.skip_while(|self_|{
      let char = self_.current_char;
      return is_valid_char_for_symbol(char);
    });
    let symbol_end = self.byte_index;
    let loc = self.end_sloc(loc);
    let diff = symbol_end - symbol_start;
    if diff == 0 {
      throw!(self.fail_with(ParseErrorKind::EmptySymbol));
    };

    return Ok(Symbol { chars_ptr: self.bytes, location: loc });
  }
}



// Raw Expr parsing
impl ParsingState {
  pub fn parse_expr(
    &mut self,
    root_indentation_depth: u32
  ) -> Maybe<RawNode> {
    let loc = self.begin_sloc();
    if self.prefix_match("*", true) {
      let loc = self.end_sloc(loc);
      let star = RawNode {
        implicit_context: None,
        kind: RawNodeRepr::Star,
        location: loc
      };
      return Ok(star)
    }
    if self.prefix_match("[|", false) {
      let wit = self.parse_witness()?;
      return Ok(wit);
    }
    if self.prefix_match("\\{", false) {
      let lambda = self.parse_lambda()?;
      return Ok(lambda);
    }
    let imp_ctx: Option<RawImplicitCtx> =
    if self.prefix_match("{", false) {
      let ctx = self.parse_implicit_context()?;
      self.skip_trivia();
      Some(ctx)
    } else { None };
    if self.prefix_match("(", false) {
      let chk = self.checkpoint();
      let lift = self.parse_lift_node();
      match lift {
        Ok(mut lift) => {
          lift.implicit_context = imp_ctx;
          return Ok(lift)
        },
        Err(_) => {
          self.backtrack_to(chk);
          self.next_char();
          let depth = self.probe_depth().max(root_indentation_depth);
          let mut expr =
            self.parse_expr(depth)?;
          guard! {
            self.prefix_match(")", true) =>
            self.fail_with(ParseErrorKind::UnterminatedSubexpr)
          };
          expr.implicit_context = imp_ctx;
          return Ok(expr);
        },
      }
    }
    let root = self.parse_symbol()?;
    let mut subexprs =
      InlineVector::<6, RawNode>::init();
    loop {
      self.skip_whitespaces();
      if self.prefix_match("\n", false) {
        let depth = self.probe_depth();
        if depth <= root_indentation_depth {
          self.byte_index -= depth as usize; break };
      }
      if self.prefix_match("(", true) {
        self.skip_trivia();
        let subexpr =
          self.parse_expr(root_indentation_depth)?;
        subexprs.append(subexpr);
        guard! {
          self.prefix_match(")", true) =>
            self.fail_with(ParseErrorKind::UnterminatedSubexpr)
        }
        continue;
      }
      if self.at_terminator() { break; }

      let terminal_subexpr = self.parse_symbol()?;
      let node = RawNode {
        implicit_context: imp_ctx,
        kind: RawNodeRepr::Ref(terminal_subexpr),
        location: terminal_subexpr.location
      };
      subexprs.append(node);
    };

    let loc = self.end_sloc(loc);

    if subexprs.is_empty() {
      let node = RawNode {
        implicit_context: imp_ctx,
        kind: RawNodeRepr::Ref(root),
        location: loc
      };
      return Ok(node);
    };
    let count = subexprs.count_items();
    let mem = self.get_mem::<RawNode>(count as usize);
    subexprs.move_content_into(mem);
    let args_ptr = ArrayPtr::init(mem, count as u8);
    let node = RawNode {
      implicit_context: imp_ctx,
      kind: RawNodeRepr::App { root, arguments: args_ptr },
      location: loc
    };
    return Ok(node)
  }
  pub fn parse_implicit_context(&mut self) -> Maybe<RawImplicitCtx> {
    guard! {
      self.prefix_match("{", true)
      => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut items =
      InlineVector::<4, (Symbol, Option<RawNode>)>::init();
    loop {
      self.skip_trivia();
      let sym = self.parse_symbol()?;
      self.skip_trivia();
      if self.prefix_match(":", true) {
        let indent = self.probe_depth();
        let expr = self.parse_expr(indent)?;
        items.append((sym, Some(expr)));
        self.skip_trivia();
      } else {
        items.append((sym, None));
      }
      match () {
        _ if self.prefix_match(",", true) => continue,
        _ if self.prefix_match("}", true) => break,
        _ => {
          throw! { self.fail_with(ParseErrorKind::UnexpectedCharacter) }
        }
      }
    }
    let count = items.count_items() as usize;
    let mem =
      self.get_mem::<(Symbol, Option<RawNode>)>(count);
    items.move_content_into(mem);
    let ctx =
      ArrayPtr::init(mem, count as u8);
    return Ok(ctx)

  }
  pub fn parse_witness(&mut self) -> Maybe<RawNode> {
    let loc = self.begin_sloc();
    guard! {
      self.prefix_match("[|", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut premises =
      InlineVector::<4, RawNode>::init();
    loop {
      let depth = self.probe_depth();
      let expr = self.parse_expr(depth)?;
      premises.append(expr);
      self.skip_trivia();
      match () {
        _ if self.prefix_match(",", true) => continue,
        _ if self.prefix_match(";", true) => break,
        _ => throw! { self.fail_with(ParseErrorKind::UnexpectedCharacter) }
      };
    };
    let depth = self.probe_depth();
    let evidence = self.parse_expr(depth)?;
    self.skip_trivia();
    guard! {
      self.prefix_match("|]", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter) };
    let loc = self.end_sloc(loc);

    let evidence_ = self.allocate(evidence);

    let count = premises.count_items();
    let concs =
      self.get_mem::<RawNode>(count as usize);
    premises.move_content_into(concs);
    let prem_ptr = ArrayPtr::init(concs, count as u8);
    let node = RawNode {
      implicit_context: None,
      kind: RawNodeRepr::Wit { premises: prem_ptr, conclusion: evidence_ },
      location: loc
    };

    return Ok(node)
  }
}


impl ParsingState {
  pub fn parse_lift_node(&mut self) -> Maybe<RawNode> {
    let loc = self.begin_sloc();
    guard! {
      self.prefix_match("(", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    }
    let mut items =
      InlineVector::<4, (Option<Symbol>, RawNode)>::init();
    loop {
      let indent = self.probe_depth();
      let chkpt = self.checkpoint();
      let maybe_expr =
        self.parse_expr(indent)?;
      self.skip_trivia();
      if self.prefix_match(":", true) {
        self.backtrack_to(chkpt);
        let ref_ = self.parse_symbol()?;
        self.skip_trivia();
        let _ = self.prefix_match(":", true);
        let depth = self.probe_depth();
        let expr = self.parse_expr(depth)?;
        items.append((Some(ref_), expr));
      } else {
        let indeed_expr = maybe_expr;
        items.append((None, indeed_expr));
      }
      self.skip_trivia();
      match () {
        _ if self.prefix_match(",", true) => continue,
        _ if self.prefix_match(")", true) => {
          self.skip_trivia(); break;
        },
        _ => throw! { self.fail_with(ParseErrorKind::UnexpectedCharacter) }
      }
    }
    let node_kind = match () {
      _ if self.prefix_match("->", true) => {
        RawKind::Fun
      },
      _ if self.prefix_match("|-", true) => {
        RawKind::Sig
      }
      _ => throw! { self.fail_with(ParseErrorKind::UnexpectedCharacter) }
    };
    let depth = self.probe_depth();
    let spine_ = self.parse_expr(depth)?;
    let spine = self.allocate(spine_);

    let loc = self.end_sloc(loc);

    let count = items.count_items();
    let items_ =
      self.get_mem::<(Option<Symbol>, RawNode)>(count as usize);
    items.move_content_into(items_);
    let items_ptr =
      ArrayPtr::init(items_, count as u8);
    let kind = match node_kind {
      RawKind::Fun => {
        RawNodeRepr::Fun { head: items_ptr, spine }
      },
      RawKind::Sig => {
        RawNodeRepr::Sigma { head: items_ptr, spine }
      },
      _ => unreachable!()
    };
    let node = RawNode {
      implicit_context: None,
      kind,
      location: loc
    };

    return Ok(node)
  }
}


impl ParsingState {
  pub fn parse_lambda(&mut self) -> Maybe<RawNode> {
    let loc = self.begin_sloc();
    guard! {
      self.prefix_match("\\{", true)
        => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut depth = self.probe_depth();
    let mut clauses =
      InlineVector::<4, RawRewriteRule>::init();
    loop {
      let clause =
        self.parse_clause(depth)?;
      clauses.append(clause);
      depth = self.probe_depth();
      if self.prefix_match("}", true) { break; }
    }
    let loc = self.end_sloc(loc);
    let count = clauses.count_items();
    let mem =
      self.get_mem::<RawRewriteRule>(count as usize);
    clauses.move_content_into(mem);
    let ptr = ArrayPtr::init(mem, count as u8);

    let node = RawNode {
      implicit_context: None,
      kind: RawNodeRepr::Lam { rewrite_rules: ptr },
      location: loc
    };
    return Ok(node)

  }
  pub fn parse_clause(
    &mut self, indentation_depth: u32
  ) -> Maybe<RawRewriteRule> {
    let loc_ = self.begin_sloc();
    guard! {
      self.prefix_match("|", true)
        => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let loc: SourceLocation;
    let mut patterns =
      InlineVector::<4, RawPattern>::init();
    loop {
      self.skip_trivia();
      let pattern = self.parse_pattern()?;
      patterns.append(pattern);
      self.skip_trivia();
      if self.prefix_match(",", true) {
        continue; }
      if self.prefix_match("=>", true) {
        loc = self.end_sloc(loc_);
        self.skip_whitespaces(); break;
      }
      throw!(self.fail_with(ParseErrorKind::UnexpectedCharacter));
    }
    let depth = self.probe_depth();
    let stencil = self.parse_expr(
      if depth == 0 { indentation_depth } else { depth } )?;
    let lhs = self.allocate(stencil);

    let count = patterns.count_items();
    let patterns_ =
      self.get_mem::<RawPattern>(count as usize);
    patterns.move_content_into(patterns_);
    let matchers_ptr =
      ArrayPtr::init(patterns_, count as u8);

    let rr = RawRewriteRule {
      matchers: matchers_ptr,
      lhs,
      location: loc
    };

    return Ok(rr);
  }
  pub fn parse_pattern(&mut self) -> Maybe<RawPattern> {
    let loc = self.begin_sloc();
    if self.prefix_match("_", true) {
      let loc = self.end_sloc(loc);
      let wk = RawPattern {
        repr: RawPatternKind::Wildcard,
        location: loc
      };
      return Ok(wk);
    }
    let root = self.parse_symbol()?;
    let mut args =
      InlineVector::<4, RawPattern>::init();
    loop {
      self.skip_trivia();
      if self.prefix_match("(", true) {
        self.skip_trivia();
        let sub_pat = self.parse_pattern()?;
        args.append(sub_pat);
        self.skip_trivia();
        guard! {
          self.prefix_match(")", true)
          => self.fail_with(ParseErrorKind::UnexpectedCharacter)
        }
        continue;
      }
      if self.at_terminator() { break; }

      let terminal_pat = self.parse_symbol()?;
      let subexpr = RawPattern {
        location: terminal_pat.location ,
        repr: RawPatternKind::Mono(terminal_pat)
      };
      args.append(subexpr);
    }
    let loc = self.end_sloc(loc);
    if args.is_empty() {
      let pat = RawPattern {
        location: loc,
        repr: RawPatternKind::Mono(root)
      };
      return Ok(pat);
    }
    let count = args.count_items();

    let mem = self.get_mem::<RawPattern>(count as usize);
    args.move_content_into(mem);
    let ptr = ArrayPtr::init(mem, count as u8);

    let pat = RawPattern {
      location: loc,
      repr: RawPatternKind::Compound { head: root, subexpressions: ptr }
    };

    return Ok(pat)
  }
}


impl ParsingState {
  pub fn parse_decl(&mut self) -> Maybe<Declaration> {
    let name = self.parse_symbol()?;
    self.skip_trivia();
    guard! {
      self.prefix_match(":", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    }
    let depth = self.probe_depth();
    let type_ =
      self.parse_expr(depth)?;
    let type__ = self.allocate(type_);

    let mut depth = self.probe_depth();
    if self.prefix_match("=", true) {
      depth = self.probe_depth();
      let value = self.parse_expr(depth)?;
      let value_ = self.allocate(value);
  
      let def_decl = Declaration {
        repr: DeclKind::RawDefinition { name, given_type: type__, value: value_ },
        participate_in_cycle_formation: false
            // This is a stub value.     👆
            // Actuall information will be established later during
            // semantic analysis phase
      };
      return Ok(def_decl)
    }
    if self.prefix_match("|", false) {
      let mut clauses =
        InlineVector::<4, RawRewriteRule>::init();
      loop {
        let clause =
          self.parse_clause(depth)?;
        clauses.append(clause);
        depth = self.probe_depth();
        if self.prefix_match("|", false) { continue; }
        else { self.byte_index -= depth as usize; break; }
      }
      let count = clauses.count_items();
      let rrs =
        self.get_mem::<RawRewriteRule>(count as usize);
      clauses.move_content_into(rrs);
      let rrs_ptr =
        ArrayPtr::init(rrs, count as u8);
      let map_decl = Declaration {
        repr: DeclKind::RawMapping {
          name, given_type: type__, rewrite_rules: rrs_ptr
        },
        participate_in_cycle_formation: false
            // This is a stub value.     👆
            // Actuall information will be established later during
            // semantic analysis phase
      };
      return Ok(map_decl)
    }
    throw!(self.fail_with(ParseErrorKind::UnexpectedCharacter));
  }
}