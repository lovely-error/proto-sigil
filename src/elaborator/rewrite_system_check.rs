
use std::{collections::{HashMap}, };

use crate::expression_trees::{better_nodes::{
  Declaration, DeclKind,  ConcretisedNode, ConcretisedNodeRepr, ConcretisedPattern, ConcretisedPatternKind, Symbol,
}, raw_syntax_nodes::SourceLocation};
use super::diagnostics::{SomeDiagnosticsDelegate, ProblemReport, Kind};



pub fn check_rewrite_system(
  declaration: Declaration,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate
) {
  if let DeclKind::WellScopedMapping {
    given_type, rewrite_rules, ..
  } = declaration.repr {
    let type_expr = unsafe { *given_type };
    if let ConcretisedNodeRepr::Arrow { .. } = type_expr.kind {}
    else {
      let problem = ProblemReport {
        kind: Kind::NonfuncTypeInFuncPos(type_expr.location)
      };
      diagnostic_delegate.report_problem(problem);
      return ;
    }

    let arity = type_expr.count_arity() ;

    let ptr = rewrite_rules.project_ptr();
    let lim = rewrite_rules.project_count();

    let mut rule_local_binders = Vec::<(BindSynthTypeShape, Symbol)>::new();

    for column in 0 .. arity {
      let mut root = BindSynthTypeShape::Variable;

      for row in 0 .. lim {
        let ptr = unsafe { *ptr.add(row) };
        let matcher = unsafe {
          *ptr.matchers.project_ptr().add(column)
        };

        synthesise_shape_from_pattern(
          matcher, diagnostic_delegate,
          &mut rule_local_binders, &mut root,);

      }




      rule_local_binders.clear();
    }



    // need to check against signature as well
  } else {
    panic!("Unexpectedly recieved nonfunction object")
  }
}

impl ConcretisedNode {
  fn count_arity(&self) -> usize {
    let mut arity = 0;
    if let ConcretisedNodeRepr::Arrow { head, .. } = self.kind {
      let count = head.project_count();
      arity += count ;
    }
    return arity
  }
}


fn synthesise_shape_from_pattern(
  pattern: ConcretisedPattern,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  binders: &mut Vec<(BindSynthTypeShape, Symbol)>,
  type_shape: &mut BindSynthTypeShape,
) {
  let ConcretisedPattern { repr, location } = pattern;

  match repr {
    ConcretisedPatternKind::Wildcard => (),
    ConcretisedPatternKind::Pt => {
      let sing = BindSynthTypeShape::Singleton;
      refine_shape(
        type_shape, &sing,
        location, diagnostic_delegate, binders)
    },
    ConcretisedPatternKind::Left(v) => {
      let mut l = BindSynthTypeShape::Variable;

      let v = unsafe { *v };
      synthesise_shape_from_pattern(
        v, diagnostic_delegate,
        binders, &mut l);

      let either = BindSynthTypeShape::Either(
        Box::new(l),
        Box::new(BindSynthTypeShape::Variable));


      refine_shape(type_shape, &either, v.location, diagnostic_delegate, binders);
    },
    ConcretisedPatternKind::Right(v) => {
      let mut r = BindSynthTypeShape::Variable;

      let v = unsafe { *v };
      synthesise_shape_from_pattern(
        v, diagnostic_delegate,
        binders, &mut r);

      let either = BindSynthTypeShape::Either(
        Box::new(BindSynthTypeShape::Variable),
        Box::new(r));


      refine_shape(type_shape, &either, v.location, diagnostic_delegate, binders);

    },
    ConcretisedPatternKind::Tuple(l, r) => {
      let l = unsafe { *l };
      let mut lt = BindSynthTypeShape::Variable;

      synthesise_shape_from_pattern(l, diagnostic_delegate, binders, &mut lt);

      let r = unsafe { *r };
      let mut rt = BindSynthTypeShape::Variable;

      synthesise_shape_from_pattern(r, diagnostic_delegate, binders, &mut rt);

      let pair = BindSynthTypeShape::Pair(
        Box::new(lt),
        Box::new(rt));

      refine_shape(type_shape, &pair, location, diagnostic_delegate, binders)
    },
    ConcretisedPatternKind::VarBinding(symbol) => {

      let fresh = BindSynthTypeShape::Variable;
      binders.push((fresh, symbol));


      let last_item_ptr = unsafe {
        binders.as_mut_ptr().add(binders.len() - 1)
        .cast::<BindSynthTypeShape>()
      };
      let sh = BindSynthTypeShape::BinderRef(last_item_ptr);

      refine_shape(type_shape, &sh, location, diagnostic_delegate, binders);

    },
  }
}

#[derive(Debug, Clone)]
pub enum BindSynthTypeShape {
  Variable,
  BinderRef(*mut BindSynthTypeShape),
  Function(Vec<Self>, Box<Self>),
  Pair(Box<Self>, Box<Self>),
  Either(Box<Self>, Box<Self>),
  Singleton,
  Sigma(Vec<Self>, Box<Self>),
  Star
}

impl BindSynthTypeShape {
  pub fn dump(&self) {
    match self {
      BindSynthTypeShape::Variable => println!("?",),
      BindSynthTypeShape::Sigma(a, b) => {
        println!("({:#?}) |- {:#?}", a, b)
      }
      BindSynthTypeShape::Function(a, b) => {
        println!("({:#?}) -> {:#?}", a, b)
      },
      BindSynthTypeShape::Pair(a, b) => {
        println!("Pair ({:#?}) ({:#?})", a, b)
      }
      BindSynthTypeShape::Either(a, b) => {
        println!("Either ({:#?}) ({:#?})", a, b)
      },
      BindSynthTypeShape::Singleton => {
        println!("Dot")
      },
      BindSynthTypeShape::Star => {
        println!("*")
      },
      BindSynthTypeShape::BinderRef(_) => todo!(),
    }
  }
}

fn refine_shape(
  lhs: &mut BindSynthTypeShape,
  rhs: &BindSynthTypeShape,
  pattern_loc: SourceLocation,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  binders: &mut Vec<(BindSynthTypeShape, Symbol)>
) {
  match (lhs, rhs) {
    (BindSynthTypeShape::BinderRef(ptr), rhs) => {
      unsafe {
        refine_shape(&mut **ptr, rhs, pattern_loc, diagnostic_delegate, binders)
      }
    }
    (lhs@BindSynthTypeShape::Variable, k) => {
      *lhs = k.clone();
    },
    (_, BindSynthTypeShape::Variable) => (),

    (BindSynthTypeShape::Pair(a, b), BindSynthTypeShape::Pair(c, d)) |
    (BindSynthTypeShape::Either(a, b), BindSynthTypeShape::Either(c, d)) => {
      refine_shape(a, c, pattern_loc, diagnostic_delegate, binders);
      refine_shape(b, d, pattern_loc, diagnostic_delegate, binders);
    },
    (BindSynthTypeShape::Sigma(l_head, l_spine), BindSynthTypeShape::Sigma(r_head, r_spine)) |
    (BindSynthTypeShape::Function(l_head, l_spine),
    BindSynthTypeShape::Function(r_head, r_spine)) => {
      if l_head.len() != r_head.len() {
        let problem = ProblemReport {
          kind: Kind::BinderShapeConflict { pattern_loc }
        };
        diagnostic_delegate.report_problem(problem);
        return
      }
      for i in 0 .. l_head.len() {
        let rhs = l_head.get_mut(i).unwrap();
        let lhs = r_head.get(i).unwrap();
        refine_shape(rhs, lhs, pattern_loc, diagnostic_delegate, binders);
      }
      refine_shape(l_spine, &r_spine, pattern_loc, diagnostic_delegate, binders);
    },
    _ => {
      let problem = ProblemReport {
        kind: Kind::BinderShapeConflict {
          pattern_loc
        }
      };
      diagnostic_delegate.report_problem(problem);
    }
  }
}




fn inspect_rhs(
  rhs: ConcretisedNode,
  binders: &mut Vec<(BindSynthTypeShape, Symbol)>,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
) {


}

fn inspect_descend(
  rhs: ConcretisedNode,
) {
  match rhs.kind {
    ConcretisedNodeRepr::Star |
    ConcretisedNodeRepr::Reference { .. } |
    ConcretisedNodeRepr::Void |
    ConcretisedNodeRepr::Singleton |
    ConcretisedNodeRepr::Pt => (),

    ConcretisedNodeRepr::App { root, arguments, origination: is_global } => {

    },
    ConcretisedNodeRepr::Wit { premises, conclusion } => {

    },
    ConcretisedNodeRepr::Sigma { head, spine } => {

    },
    ConcretisedNodeRepr::Arrow { head, spine, performs_introspection } => {

    },
    ConcretisedNodeRepr::Lam { rewrite_rules } => {

    },
    ConcretisedNodeRepr::Either(l, r) |
    ConcretisedNodeRepr::Pair(l, r) => {



    },
    ConcretisedNodeRepr::Tuple(l, r) => {
      let l = unsafe { *l };
      inspect_descend(l);
      let r = unsafe { *r };
      inspect_descend(r);

    },
    ConcretisedNodeRepr::Left(v) |
    ConcretisedNodeRepr::Right(v) => {
      let v = unsafe { *v };
      inspect_descend(v);
    },
  }
}