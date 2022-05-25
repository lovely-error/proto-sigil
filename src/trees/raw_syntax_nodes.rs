
use std::ptr::{addr_of_mut, addr_of};

use crate::parser::{
  node_allocator::EntangledPtr, parser::symbol::Symbol};


pub struct SourceLocation {
  primary_offset: u32,
  secondary_offset: u32
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
    let mut val = (ptr as u64) << 8;
    unsafe {
      addr_of_mut!(val).cast::<RawKind>().write(kind);
    }
    return Self(val);
  }
  pub fn init_app_node(kind: RawKind, ptr: *mut (), arg_num:usize) -> Self {
    if arg_num > u8::MAX as usize { panic!("Too many args has been given") }
    let mut val = (ptr as u64) << 8;
    val += arg_num as u64;
    val = val << 8;
    val += kind as u64;
    return Self(val);
  }
  pub fn project_tag(&self) -> RawKind {
    unsafe {
      return *addr_of!(*self).cast::<RawKind>();
    }
  }
  pub fn project_ptr(&self) -> *mut () {
    if let RawKind::App_ArgsInline |
           RawKind::App_ArgsInVec |
           RawKind::App_ArgsInSlab = self.project_tag() {
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

// Pi and Sigma
pub struct LiftNode {
  spine_node: EntangledPtr,
  sloc_data: SourceLocation,
}

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