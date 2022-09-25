
use std::{intrinsics::transmute};

use crate::parser::{
  node_allocator::SomeEntangledPtr, };

use super::better_nodes::Symbol;

pub trait Locatable {
  type Location: Eq + Copy;
  fn compute_location(&self) -> Self::Location;
}

#[derive(Clone, Copy, Debug)]
pub struct SourceLocation {
  pub primary_offset: u32,
  pub secondary_offset: u32
}

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
  pub fn mark_context_presence(&mut self) {
    self.0 = self.0 | 0x80
  }
  pub fn init_null() -> Self { Self(0) }
  pub fn is_null(&self) -> bool { self.0 == 0 }
  pub fn init_counted_node(
    kind: RawKind, ptr: *mut (), arg_num:usize
  ) -> Self {
    if arg_num > u8::MAX as usize { panic!("Too many args have been given") }
    let mut val = (ptr as u64) << 8;
    val += arg_num as u64;
    val = val << 8;
    val += kind as u64;
    return Self(val);
  }
  pub fn project_presan_tag(&self) -> RawKind {
    let head = self.0 & 0x7F;
    return unsafe { transmute(head as u8) }
  }
  pub fn project_ptr(&self) -> *mut () {
    if let RawKind::App_ArgsInline |
           RawKind::App_ArgsInVec |
           RawKind::Lam |
           RawKind::App_ArgsInSlab |
           RawKind::Fun |
           RawKind::Wit |
           RawKind::Sig = self.project_presan_tag() {
      return (self.0 >> 16) as *mut _ ;
    };
    return (self.0 >> 8) as *mut _ ;
  }
  pub fn project_count(&self) -> u8 {
    (self.0 >> 8) as u8
  }
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
pub struct RefPatternNode {
  pub name: Symbol
}

#[derive(Debug, Clone, Copy)]
pub struct RewriteRule {
  pub pattern_count: u16,
  pub patterns: SomeEntangledPtr,
  pub stencil: ExprPtr
}
#[derive(Debug, Clone, Copy)]
pub struct Lambda {
  pub clause_count: u16,
  pub rules: SomeEntangledPtr,
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
  pub fn deref_as_map(&self) -> Mapping {
    let ptr = self.project_ptr();
    let map = unsafe { *ptr.cast::<Mapping>() };
    return map;
  }
  pub fn deref_as_defn(&self) -> Definition {
    let ptr = self.project_ptr();
    let defn = unsafe { *ptr.cast::<Definition>() };
    return defn;
  }
}

impl DeclPtr {
  pub fn project_name(&self) -> Symbol { unsafe {
    let ptr = self.project_ptr();
    match self.project_tag() {
      DeclKind::Definition => {
        let defn = *ptr.cast::<Definition>();
        return defn.name
      },
      DeclKind::Mapping => {
        let map = *ptr.cast::<Mapping>();
        return map.name;
      },
    }
  } }
  // pub fn project_location(&self) -> SourceLocation {
  //   let ptr = self.project_ptr();
  //   match self.project_tag() {
  //     DeclKind::Definition => {
  //       let defn = *ptr.cast::<Definition>();
  //       return defn.;
  //     },
  //     DeclKind::Mapping => todo!(),
  //   }
  // }
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
  pub clauses: SomeEntangledPtr
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
  pub head: SomeEntangledPtr,
  pub ctx_ptr: RawCtxPtr,
}

#[derive(Debug, Clone, Copy)]
pub struct LiftNodeItem {
  pub name: Option<Symbol>,
  pub val: ExprPtr,
}

#[derive(Debug, Clone, Copy)]
pub struct RawCtxPtr(u64);
impl RawCtxPtr {
  pub fn init(item_count: u8, ptr: *mut ()) -> Self {
    Self(((ptr as u64) << 8) + item_count as u64)
  }
  pub fn project_count(&self) -> u8 {
    unsafe { transmute(self.0 as u8) }
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 8) as *mut _
  }
  pub fn init_null() -> Self {
    Self(0)
  }
  pub fn is_null(&self) -> bool {
    self.0 == 0
  }
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessNodeIndirect {
  pub sloc_data: SourceLocation,
  pub seal: ExprPtr,
  pub items: SomeEntangledPtr,
}

