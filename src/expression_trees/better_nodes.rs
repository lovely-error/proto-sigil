use std::{marker::PhantomData, intrinsics::transmute, hash::Hash};

use crate::{support_structures::homemade_slice::Slice, parser::node_allocator::EntagledPtr};

use super::raw_syntax_nodes::SourceLocation;

#[derive(Debug, Clone, Copy)]
pub struct Symbol {
  pub chars_ptr: Slice<u8>,
  pub location: SourceLocation
}

impl Hash for Symbol {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let data = self.materialise_name();
    state.write(data.as_bytes())
  }
}
impl Eq for Symbol {}
impl PartialEq for Symbol {
  fn eq(&self, other: &Self) -> bool {
    // fix by interning
    let l = self.materialise_name();
    let r = other.materialise_name();
    return l == r
  }
}

impl Symbol {
  pub fn materialise_name<'a>(&self) -> &'a str {
    let Slice { source_data, span } = self.chars_ptr;
    let slice = unsafe {
      std::slice::from_raw_parts(source_data, span as usize)
    };
    let SourceLocation { primary_offset, secondary_offset } = self.location;
    let slice = &slice[primary_offset as usize .. secondary_offset as usize ];
    let str = std::str::from_utf8(slice).unwrap();
    return str
  }
}

#[derive(Debug, Clone, Copy)]
pub struct ArrayPtr<T>(u64, PhantomData<T>);
impl <T> ArrayPtr<T> {
  pub fn init(ptr: *mut T, count: u8) -> Self {
    let sized = ((ptr as u64) << 8) + count as u64;
    return Self(sized, PhantomData)
  }
  pub fn project_ptr(&self) -> *mut T {
    (self.0 >> 8) as *mut T
  }
  pub fn project_count(&self) -> usize {
    (self.0 as u8) as usize
  }
}
impl <T> ArrayPtr<T> where T: Copy {
  pub fn cast<K>(&self) -> ArrayPtr<K> {
    unsafe { transmute(*self) }
  }
  pub fn for_each_value(&self, mut fun: impl FnMut(T)) {
    let ptr = self.project_ptr();
    let end_index = self.project_count();
    for i in 0 .. end_index {
      let node = unsafe { *ptr.add(i) };
      fun(node)
    }
  }
}

#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum NodeKind {
  RawNode, CheckedNode
}
#[derive(Debug, Clone, Copy)]
pub struct SomeExprPtr(u64);
impl SomeExprPtr {
  pub fn init(ptr: *const (), kind: NodeKind) -> Self {
    let pd = ((ptr as u64) << 8) + kind as u64;
    return Self(pd)
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 8) as *mut ()
  }
  pub fn project_kind(&self) -> NodeKind {
    unsafe { transmute(self.0 as u8) }
  }
}

pub type RawImplicitCtx = ArrayPtr<(Symbol, Option<RawNode>)>;

#[derive(Debug, Clone, Copy)]
pub enum RawNodeRepr {
  Star,
  Ref(Symbol),
  App {
    root: Symbol,
    arguments: ArrayPtr<RawNode>,
  },
  Wit {
    premises: ArrayPtr<RawNode>,
    conclusion: *mut RawNode
  },
  Fun {
    head: ArrayPtr<(Option<Symbol>, RawNode)>,
    spine: *mut RawNode
  },
  Sigma {
    head: ArrayPtr<(Option<Symbol>, RawNode)>,
    spine: *mut RawNode
  },
  Lam {
    rewrite_rules: ArrayPtr<RawRewriteRule>
  }
}

#[derive(Debug, Clone, Copy)]
pub struct RawNode {
  pub kind: RawNodeRepr,
  pub location: SourceLocation,
  pub implicit_context: Option<RawImplicitCtx>
}

#[derive(Debug, Clone, Copy)]
pub struct RawRewriteRule {
  pub matchers: ArrayPtr<RawPattern>,
  pub lhs: *mut RawNode,
  pub location: SourceLocation
}

#[derive(Debug, Clone, Copy)]
pub enum DeclKind {
  RawMapping {
    name: Symbol,
    given_type: *mut RawNode,
    rewrite_rules: ArrayPtr<RawRewriteRule>
  },
  RawDefinition {
    name: Symbol,
    given_type : *mut RawNode,
    value: *mut RawNode,
  },
  WellScopedMapping {
    name: Symbol,
    given_type: *mut ConcretisedNode,
    rewrite_rules: ArrayPtr<ConcretisedRewriteRule>
  },
  WellScopedDefinition {
    name: Symbol,
    given_type : *mut ConcretisedNode,
    value: *mut ConcretisedNode
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Declaration {
  pub repr: DeclKind,
  pub participate_in_cycle_formation: bool,
}


#[derive(Debug, Clone, Copy)]
pub struct RawPattern {
  pub repr: RawPatternKind,
  pub location: SourceLocation,
}

#[derive(Debug, Clone, Copy)]
pub enum RawPatternKind {
  Wildcard,
  Compound {
    head: Symbol,
    subexpressions: ArrayPtr<RawPattern>
  },
  Mono(Symbol)
}


pub type ConcretisedImplicitCtx = ArrayPtr<(Symbol, Option<ConcretisedNode>)>;

#[derive(Debug, Clone, Copy)]
pub enum Origin {
  GlobalScope, PatternBinding, ContextBinding
}

#[derive(Debug, Clone, Copy)]
pub enum ConcretisedNodeRepr {
  Star,
  Reference {
    ref_: Symbol,
    origination: Origin
  },
  App {
    root: Symbol,
    arguments: ArrayPtr<ConcretisedNode>,
    origination: Origin
  },
  Wit {
    premises: ArrayPtr<ConcretisedNode>,
    conclusion: *mut ConcretisedNode,
  },
  Sigma {
    head: ArrayPtr<(Option<Symbol>, ConcretisedNode)>,
    spine: *mut ConcretisedNode,
  },
  Arrow {
    head: ArrayPtr<(Option<Symbol>, ConcretisedNode)>,
    spine: *mut ConcretisedNode,
    performs_introspection: bool,
  },
  Lam {
    rewrite_rules: ArrayPtr<ConcretisedRewriteRule>
  },
  Void,
  Singleton,
  Pt,
  Pair(*mut ConcretisedNode, *mut ConcretisedNode,),
  Tuple(*mut ConcretisedNode, *mut ConcretisedNode,),
  Either(*mut ConcretisedNode, *mut ConcretisedNode,),
  Left(*mut ConcretisedNode,),
  Right(*mut ConcretisedNode,),
}

#[derive(Debug, Clone, Copy)]
pub struct ConcretisedNode {
  pub kind: ConcretisedNodeRepr,
  pub location: SourceLocation,
  pub implicit_context: Option<ConcretisedImplicitCtx>
}

#[derive(Debug, Clone, Copy)]
pub struct ConcretisedRewriteRule {
  pub matchers: ArrayPtr<ConcretisedPattern>,
  pub rhs: *mut ConcretisedNode,
  pub location: SourceLocation
}

#[derive(Debug, Clone, Copy)]
pub struct ConcretisedPattern {
  pub repr: ConcretisedPatternKind,
  pub location: SourceLocation,
}

#[derive(Debug, Clone, Copy)]
pub enum ConcretisedPatternKind {
  Wildcard,
  Pt,
  Left(*mut ConcretisedPattern),
  Right(*mut ConcretisedPattern),
  Tuple(*mut ConcretisedPattern, *mut ConcretisedPattern),
  VarBinding(Symbol)
}


#[derive(Debug, Clone, Copy)]
pub struct EvaluableExprPtr(u64);
impl EvaluableExprPtr {
  // const TAG_SIZE : u64 = 5;
  pub fn init(ptr: *mut (), kind: EvaluableExprRepr) -> Self {
    let ptr = ((ptr as u64) << 5) + (kind as u64);
    return Self(ptr)
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 5) as *mut ()
  }
  pub fn project_kind(&self) -> EvaluableExprRepr {
    let proj = (self.0 as u8) & 0xF;
    return unsafe { transmute(proj) }
  }
  pub fn is_marked_for_lazy_copying(&self) -> bool {
    let mask = 1u64 << 4;
    let val = self.0 & mask;
    return val != 0
  }
  pub fn mark_for_lazy_copying(&mut self) {
    let mask = 1u64 << 4;
    let marked = self.0 | mask;
    self.0 = marked
  }
}

pub struct TypePtr(u64);
impl TypePtr {

}

pub enum EvalExprTypeType {
  Mono, Multi, Unknown
}


pub struct Reference {
  pub symbol: Symbol,
  pub is_global: bool,
}
pub struct Application {
  pub root: Symbol,
  pub arguments: (),
  pub is_global: bool
}
pub struct Witness {
  pub premises: (),
  pub conclusion: (),
  pub type_expr: (),
}
pub struct Lift {
  pub head: ArrayPtr<(Option<Symbol>, ())>,
  pub spine: (),
}
pub struct Lambda {
  pub rewrite_rules: ArrayPtr<ConcretisedRewriteRule>,
  pub type_expr: (),
}
pub struct Node2 {
  pub left: EntagledPtr<EvaluableExprPtr>,
  pub right: EntagledPtr<EvaluableExprPtr>,
  pub type_expr: (),
}
pub struct Node1 {
  pub value: (),
  pub type_expr: (),
}

pub enum EvaluableExprRepr {
  Star, Reference, App, Wit, Sigma, Pi , Lam , Void,
  Singleton, Pt, Pair, Tuple, Either, Left, Right,
}
