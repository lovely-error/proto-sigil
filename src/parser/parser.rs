
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::{size_of};
use std::ptr::{null_mut};
use std::str;
use crate::parser::parser::symbol::{Symbol};
use crate::{ throw, guard };
use crate::support_structures::monad::{
  fail_with_aux_gen_ctx_intro};
use crate::parser::node_allocator::NodeSlabSizeInBytes;

#[derive(Clone, Copy, Debug)]
pub struct SourceLocation {
  pub primary_offset: u32,
  pub secondary_offset: u16
}

#[derive(Debug, Clone, Copy)]
pub struct TempSlocInfo {
  pub primary_offset: u32
}

pub struct ParsingState<'chars> {
  bytes: &'chars [u8],
  current_char: u32,
  pub byte_index: usize,
    // Todo: figure out how to expose fields to testing code
  line_number: u32,
  slab_alloc_64: Option<LinearAllocator<64>>,
  total_char_count: u32,
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

pub mod symbol {
  use std::{str::FromStr};

  use super::SourceLocation;

  #[derive(Debug, Copy, Clone)]
  pub struct Symbol {
    pub hash: u64, pub sloc: SourceLocation,
  }
  impl Symbol {
  pub fn normalise_name
    (&self, chars: &[u8]) -> String {
      let slice =
        &chars[
          self.sloc.primary_offset as usize ..
          self.sloc.secondary_offset as usize];
      let str =
        std::str::from_utf8(slice).unwrap();
      return String::from_str(str).unwrap();
    }
  }
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
impl<'v> ParsingState<'v> {
  pub fn init(char_ptr: &'v [u8]) -> Self {
    Self {
      byte_index: 0,
      bytes: char_ptr,
      current_char: unsafe { *char_ptr.get_unchecked(0) as u32 },
      line_number: 1,
      slab_alloc_64: None,
      total_char_count: 0
    }
  }
}

/// Accessors
impl ParsingState<'_> {
  fn get_slab64(&mut self) -> *mut () {
    if let None = self.slab_alloc_64 {
      //unlikely(true);
      let local_mem_man =
        LinearAllocator::<64>::init();
      self.slab_alloc_64 = Some(local_mem_man);
    }
    if let Some(ref mut memman) = self.slab_alloc_64 {
      return memman.get_slot();
    }
    unreachable!()
  }
  fn get_mem(&mut self, byte_count: usize) -> *mut () {
    if let None = self.slab_alloc_64 {
      let fresh_mem_man = LinearAllocator::<64>::init();
      self.slab_alloc_64 = Some(fresh_mem_man);
    }
    if let Some(ref mut mem_man) = self.slab_alloc_64 {
      return mem_man.get_contiguos_mem(byte_count)
    }
    unreachable!()
  }
}

/// Preliminary methods
impl ParsingState<'_> {
  fn fail_with(&self, error: ParseErrorKind) -> ParseError {
    ParseError { absolute_offset: self.byte_index, kind: error }
  }
  pub fn no_more_chars(&self) -> bool {
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
impl ParsingState<'_> {
  pub fn parse_symbol(&mut self) -> Maybe<symbol::Symbol> {
    let sloc_begin = self.begin_sloc();
    let symbol_start = self.byte_index;
    self.skip_while(|self_|{
      let char = self_.current_char;
      return is_valid_char_for_symbol(char);
    });
    let symbol_end = self.byte_index;
    let diff = symbol_end - symbol_start;
    let sloc = self.end_sloc(sloc_begin);
    if diff == 0 {
      throw!(self.fail_with(ParseErrorKind::EmptySymbol));
    };
    let mut hasher = DefaultHasher::new();
    let str = &self.bytes[symbol_start .. symbol_end];
    str.hash(&mut hasher);
    let hash = hasher.finish();
    return Ok(Symbol { hash, sloc });
  }
}


use crate::trees::raw_syntax_nodes::{
  RawKind, RefNode, AppNodeArgsInline, AppNodeIndirectSmall,
  ExprPtr, PatternExprPtr,
  CompoundPatternNode_ArgsInline, RefPatternNode,
  CompoundPatternNode_ArgsIndiSlab, RewriteRule, Definition, DeclPtr, Mapping,
  LiftNodeItem, LiftNode, RawCtxPtr, WitnessNodeIndirect};
use crate::support_structures::mini_vector::InlineVector;
use super::node_allocator::{LinearAllocator, EntangledPtr};


// Raw Expr parsing
impl ParsingState<'_> {
  pub fn parse_expr(
    &mut self,
    root_indentation_depth: u32
  ) -> Maybe<ExprPtr> { unsafe {
    if self.prefix_match("*", true) {
      let star = ExprPtr::init(RawKind::Star, null_mut());
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
    let imp_ctx: RawCtxPtr =
    if self.prefix_match("{", false) {
      let ctx = self.parse_implicit_context()?;
      self.skip_trivia();
      ctx
    } else { RawCtxPtr::init_null() };
    if self.prefix_match("(", false) {
      let chk = self.checkpoint();
      let lift = self.parse_lift_node(imp_ctx);
      match lift {
        Ok(lift) => return Ok(lift),
        Err(_) => {
          self.backtrack_to(chk);
          self.next_char();
          let depth = self.probe_depth().max(root_indentation_depth);
          let expr =
            self.parse_expr(depth)?;
          guard! {
            self.prefix_match(")", true) =>
            self.fail_with(ParseErrorKind::UnterminatedSubexpr)
          };
          return Ok(expr);
        },
      }
    }
    let loc = self.begin_sloc();
    let root = self.parse_symbol()?;
    let mut subexprs =
      InlineVector::<4, ExprPtr>::init();
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
      let mem_ptr = self.get_slab64();
      let data = &mut *mem_ptr.cast::<RefNode>();
      data.name = terminal_subexpr;
      data.ctx_ptr = RawCtxPtr::init_null();
      let expr_ptr = ExprPtr::init(RawKind::Ref, mem_ptr);
      subexprs.append(expr_ptr);
    };
    let loc = self.end_sloc(loc);
    let this_node_ptr =
      self.get_slab64();
    if subexprs.is_empty() {
      let node_ptr = &mut *this_node_ptr.cast::<RefNode>();
      node_ptr.name = root;
      node_ptr.ctx_ptr = imp_ctx;
      let expr_ptr = ExprPtr::init(RawKind::Ref, this_node_ptr);
      return Ok(expr_ptr);
    };
    let count = subexprs.count_items();
    if count <= 2 {
      let mut data =
        &mut *this_node_ptr.cast::<AppNodeArgsInline>();
      data.name = root;
      data.sloc_data = loc;
      data.ctx_ptr = imp_ctx;
      subexprs.move_content_into(data.args.as_mut_ptr());
      let expr_ptr = ExprPtr::init_counted_node(
        RawKind::App_ArgsInline, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };
    if count <= (NodeSlabSizeInBytes / size_of::<ExprPtr>()) as u32 {
      let invocation_data = self.get_slab64();
      subexprs.move_content_into(invocation_data.cast());
      let ptr_to_arg_ptrs = EntangledPtr::from_ptr_pair(
        this_node_ptr, invocation_data)
        .expect("Values should be close enough");
      let node =
        &mut *this_node_ptr.cast::<AppNodeIndirectSmall>();
      node.name = root;
      node.args = ptr_to_arg_ptrs;
      node.sloc_data = loc;
      node.ctx_ptr = imp_ctx;
      let expr_ptr = ExprPtr::init_counted_node(
        RawKind::App_ArgsInSlab, this_node_ptr, count as usize);
      return Ok(expr_ptr);
    };

    todo!("too many args :)");
  } }
  pub fn parse_implicit_context(&mut self) -> Maybe<RawCtxPtr> {
    guard! {
      self.prefix_match("{", true)
      => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut items =
      InlineVector::<4, (Symbol, ExprPtr)>::init();
    loop {
      self.skip_trivia();
      let sym = self.parse_symbol()?;
      self.skip_trivia();
      if self.prefix_match(":", true) {
        let indent = self.probe_depth();
        let expr = self.parse_expr(indent)?;
        items.append((sym, expr));
        self.skip_trivia();
      } else {
        items.append((sym, ExprPtr::init_null()));
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
    let stor_cap_req = size_of::<(Symbol, ExprPtr)>() * count;
    let mem = self.get_mem(stor_cap_req);
    items.move_content_into(mem.cast());
    let ctx_ptr = RawCtxPtr::init(count as u8, mem);
    return Ok(ctx_ptr)

  }
  pub fn parse_witness(&mut self) -> Maybe<ExprPtr> {
    let loc = self.begin_sloc();
    guard! {
      self.prefix_match("[|", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut items =
      InlineVector::<4, ExprPtr>::init();
    loop {
      let depth = self.probe_depth();
      let expr = self.parse_expr(depth)?;
      items.append(expr);
      self.skip_trivia();
      match () {
        _ if self.prefix_match(",", true) => continue,
        _ if self.prefix_match(";", true) => break,
        _ => throw! { self.fail_with(ParseErrorKind::UnexpectedCharacter) }
      };
    };
    let depth = self.probe_depth();
    let focus = self.parse_expr(depth)?;
    self.skip_trivia();
    guard! {
      self.prefix_match("|]", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter) };
    let loc = self.end_sloc(loc);
    let this_node = self.get_slab64();
    let count = items.count_items();
    if count as usize <= NodeSlabSizeInBytes / size_of::<ExprPtr>() {
      let item_storage = self.get_slab64();
      items.move_content_into(item_storage.cast());
      let ptr =
        EntangledPtr::from_ptr_pair(
          this_node.cast(), item_storage.cast())
          .unwrap();
      let mut data = unsafe {
        &mut *this_node.cast::<WitnessNodeIndirect>()
      };
      data.seal = focus;
      data.sloc_data = loc;
      data.items = ptr;
      let expr_ptr =
        ExprPtr::init_counted_node(
          RawKind::Wit, this_node, count as usize);
      return Ok(expr_ptr)
    }

    todo!()
  }
}


impl ParsingState<'_> {
  pub fn parse_lift_node(&mut self, imp_ctx: RawCtxPtr) -> Maybe<ExprPtr> {
    guard! {
      self.prefix_match("(", true) =>
      self.fail_with(ParseErrorKind::UnexpectedCharacter)
    }
    let mut items =
      InlineVector::<4, LiftNodeItem>::init();
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
        items.append(LiftNodeItem { name: Some(ref_), val: expr });
      } else {
        let indeed_expr = maybe_expr;
        items.append(LiftNodeItem { name: None, val: indeed_expr });
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
    let spine = self.parse_expr(depth)?;

    let count = items.count_items();
    let slab = self.get_slab64();
    if count as usize <=
    NodeSlabSizeInBytes / size_of::<LiftNodeItem>() { unsafe {
      items.move_content_into(slab.cast());
      let node_ptr = self.get_slab64();
      let data = &mut *node_ptr.cast::<LiftNode>();
      data.head = EntangledPtr::from_ptr_pair(
        node_ptr, slab).unwrap();
      data.spine_expr = spine;
      data.ctx_ptr = imp_ctx;
      let node : ExprPtr = match node_kind {
        RawKind::Fun => ExprPtr::init_counted_node(
          RawKind::Fun, node_ptr, count as _),
        RawKind::Sig => ExprPtr::init_counted_node(
          RawKind::Sig, node_ptr, count as _),
        _ => unreachable!()
      };
      return Ok(node);
    } }

    todo!()
  }
}


impl ParsingState<'_> {
  pub fn parse_lambda(&mut self) -> Maybe<ExprPtr> {
    guard! {
      self.prefix_match("\\{", true)
        => self.fail_with(ParseErrorKind::UnexpectedCharacter)
    };
    let mut depth = self.probe_depth();
    let slab = self.get_slab64();
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
      let arg_slab = self.get_slab64();
      clauses.move_content_into(arg_slab.cast());
      let node_ptr =
        ExprPtr::init_counted_node(
          RawKind::Lam, slab, count as usize);
      return Ok(node_ptr);
    }

    todo!()
  }
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
      self.skip_trivia();
      let pattern = self.parse_pattern()?;
      patterns.append(pattern);
      self.skip_trivia();
      if self.prefix_match(",", true) {
        continue; }
      if self.prefix_match("=>", true) {
        self.skip_whitespaces(); break; }
      throw!(self.fail_with(ParseErrorKind::UnexpectedCharacter));
    }
    let depth = self.probe_depth();
    let stencil = self.parse_expr(
      if depth == 0 { indentation_depth } else { depth } )?;

    let count = patterns.count_items();
    let pats = self.get_slab64();
    patterns.move_content_into(pats.cast());

    if count as usize <= NodeSlabSizeInBytes / size_of::<PatternExprPtr>() {
      let slab = self.get_slab64();
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
      let slab = self.get_slab64();
      let node = &mut *slab.cast::<RefPatternNode>();
      node.name = terminal_pat;
      let expr = PatternExprPtr::init_singular(slab);
      args.append(expr);
    }
    let slab = self.get_slab64();
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
      let indi_args = self.get_slab64();
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
    self.skip_trivia();
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
        let slab = self.get_slab64();
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
        let slab = self.get_slab64();
        let slab_ = slab.cast::<EntangledPtr>();
        for i in 0 .. count {
          let rr_ptr = clauses.get_ref(i);
          let entp =
            EntangledPtr::from_ptr_pair(
              slab, (*rr_ptr).cast()).unwrap();
          *slab_.add(i as usize) = entp;
        }
        let map_decl_ptr = self.get_slab64();
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
  pub fn run_parsing(&mut self) -> Maybe<Vec<DeclPtr>> {
    let mut decls = Vec::new();
    loop {
      self.skip_trivia();
      if self.no_more_chars() { break; }
      let decl = self.parse_decl()?;
      decls.push(decl);
    }
    return Ok(decls);
  }
}