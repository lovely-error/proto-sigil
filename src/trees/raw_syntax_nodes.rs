
use std::{intrinsics::transmute};

use crate::parser::{
  node_allocator::EntangledPtr, parser::symbol::Symbol};


#[repr(u8)] #[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawKind {
  Ref,
  App_ArgsInSlab, App_ArgsInline, App_ArgsInVec,
  Lam, Wit, Fun, Sig, Star
}

#[repr(u8)] #[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
  Star, Empty, Dot, Either, Pair, Arrow, Pi, Sigma,
  pt, left, right, tuple, lambda, witness
}

#[derive(Debug, Copy, Clone)]
pub struct ExprPtr(u64);
impl ExprPtr {
  pub fn init(kind: RawKind, ptr: *mut ()) -> Self {
    let val = ((ptr as u64) << 8) + kind as u64;
    return Self(val);
  }
  pub fn init_counted_node(
    kind: RawKind, ptr: *mut (), arg_num:usize
  ) -> Self {
    if arg_num > u8::MAX as usize { panic!("Too many args has been given") }
    let mut val = (ptr as u64) << 8;
    val += arg_num as u64;
    val = val << 8;
    val += kind as u64;
    return Self(val);
  }
  pub fn project_tag(&self) -> RawKind {
    unsafe { transmute(self.0 as u8) }
  }
  pub fn project_ptr(&self) -> *mut () {
    if let RawKind::App_ArgsInline |
           RawKind::App_ArgsInVec |
           RawKind::Lam |
           RawKind::App_ArgsInSlab |
           RawKind::Fun |
           RawKind::Sig = self.project_tag() {
      return (self.0 >> 16) as *mut _ ;
    };
    return (self.0 >> 8) as *mut _ ;
  }
  pub fn project_count(&self) -> u8 {
    (self.0 >> 8) as u8
  }
}


// All nontrivial nodes have same layout: 64 bytes
pub type GenericNodeData = [u64 ; 8];

#[derive(Debug, Copy, Clone)]
pub struct AppNodeArgsInline {
  pub name: Symbol,
  //sloc_data: SourceLocation,
  pub args: [ExprPtr ; 4]
}

#[derive(Debug, Copy, Clone)]
pub struct AppNodeIndirectSmall {
  pub name: Symbol,
  //sloc_data: SourceLocation,
  pub args: EntangledPtr
}

#[derive(Debug, Copy, Clone)]
pub struct RefNode {
  pub name: Symbol,
  //pub sloc_data: SourceLocation
}


pub struct AppNodeVec {
  pub name: Symbol,
  pub args: Vec<ExprPtr>,
}



#[repr(u8)]
pub enum PatternKind {
  Wildcard, Compound_Inlined, Compound_Indirect, Compound_Huge, Singular
}

#[derive(Debug, Copy, Clone)]
pub struct PatternExprPtr(u64);
impl PatternExprPtr {
  pub fn init_wildcard() -> Self {
    let ptr = 0u64;
    return Self(ptr + PatternKind::Wildcard as u64);
  }
  pub fn init_compound_inlined(node_ptr: *mut (), arg_count: u8) -> Self {
    let mut ptr = (node_ptr as u64) << 8;
    ptr += arg_count as u64;
    ptr = ptr << 8;
    ptr += PatternKind::Compound_Inlined as u64;
    return Self(ptr);
  }
  pub fn init_compound_indirect(node_ptr: *mut (), arg_count: u8) -> Self {
    let mut ptr = (node_ptr as u64) << 8;
    ptr += arg_count as u64;
    ptr = ptr << 8;
    ptr += PatternKind::Compound_Indirect as u64;
    return Self(ptr);
  }
  pub fn init_singular(node_ptr: *mut ()) -> Self {
    let ptr = (node_ptr as u64) << 8;
    let tagged = ptr + PatternKind::Singular as u64;
    return Self(tagged);
  }
  pub fn project_ptr(&self) -> *mut () {
    let kind = self.project_tag();
    if let PatternKind::Compound_Inlined |
           PatternKind::Compound_Indirect = kind {
      return (self.0 >> 16) as *mut _
    }
    return (self.0 >> 8) as *mut _
  }
  pub fn project_tag(&self) -> PatternKind {
    unsafe { transmute(self.0 as u8) }
  }
  pub fn project_count(&self) -> u8 {
    let ptr = self.0 >> 8;
    return unsafe { transmute(ptr as u8) }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct CompoundPatternNode_ArgsInline {
  pub name: Symbol,
  pub args: [PatternExprPtr ; 4]
}

#[derive(Debug, Clone, Copy)]
pub struct CompoundPatternNode_ArgsIndiSlab {
  pub name: Symbol,
  pub args: EntangledPtr
}

#[derive(Debug, Clone, Copy)]
pub struct RefPatternNode {
  pub name: Symbol
}

#[derive(Debug, Clone, Copy)]
pub struct RewriteRule {
  pub pattern_count: u16,
  pub patterns: EntangledPtr,
  pub stencil: ExprPtr
}
#[derive(Debug, Clone, Copy)]
pub struct Lambda {
  pub clause_count: u16,
  pub rules: EntangledPtr,
}




#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum DeclKind {
  Definition, Mapping
}
#[derive(Debug, Clone, Copy)]
pub struct DeclPtr(u64);
impl DeclPtr {
  pub fn init_def(def_ptr: *mut ()) -> Self {
    let ptr =
      ((def_ptr as u64) << 8) + DeclKind::Definition as u64;
    return Self(ptr);
  }
  pub fn init_map(def_ptr: *mut (), clause_count: u8) -> Self {
    let ptr =
      ((((def_ptr as u64) << 8) + (clause_count as u64))
      << 8) + DeclKind::Mapping as u64;
    return Self(ptr);
  }
  pub fn project_tag(&self) -> DeclKind {
    unsafe { transmute(self.0 as u8) }
  }
  pub fn project_ptr(&self) -> *mut () {
    let tag = self.project_tag();
    if let DeclKind::Mapping = tag {
      return (self.0 >> 16) as *mut _;
    }
    return (self.0 >> 8) as *mut _
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Definition {
  pub name: Symbol,
  pub type_: ExprPtr,
  pub value: ExprPtr,
}
#[derive(Debug, Clone, Copy)]
pub struct Mapping {
  pub name: Symbol,
  pub type_: ExprPtr,
  pub clauses: EntangledPtr
}

#[derive(Debug, Clone, Copy)]
pub enum LiftNodeKind {
  Sigma, Pi
}
#[derive(Debug, Clone, Copy)]
pub struct LiftNodePtr(u64);
impl LiftNodePtr {
  pub fn init_sigma(node_ptr: *mut (), arg_count: u8) -> Self {
    let ptr =
      ((((node_ptr as u64) << 8) + (arg_count as u64)) << 8)
      + LiftNodeKind::Sigma as u64;
    return Self(ptr);
  }
  pub fn init_pi(node_ptr: *mut (), arg_count: u8) -> Self {
    let ptr =
      ((((node_ptr as u64) << 8) + (arg_count as u64)) << 8)
      + LiftNodeKind::Pi as u64;
    return Self(ptr);
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 16) as *mut _
  }
  pub fn project_count(&self) -> u8 {
    unsafe { transmute((self.0 >> 8) as u8) }
  }
}
#[derive(Debug, Clone, Copy)]
pub struct LiftNode {
  pub spine_expr: ExprPtr,
  pub head: EntangledPtr,
}

#[derive(Debug, Clone, Copy)]
pub struct LiftNodeItem {
  pub name: Option<Symbol>,
  pub val: ExprPtr,
}