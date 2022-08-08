
use std::collections::HashMap;

use crate::expression_trees::better_nodes::{
  ConcretisedNode, ConcretisedNodeRepr, Symbol, Declaration, ArrayPtr, DeclKind, Origin
};
use super::{
  diagnostics::{SomeDiagnosticsDelegate, ProblemReport, Kind}, environment::PasteboardTable
};



pub fn check_static_shape(
  type_expr: ConcretisedNode,
  term_expr: ConcretisedNode,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  declarations_table: &PasteboardTable<Symbol, Declaration>
) {
  match (type_expr.kind, term_expr.kind) {
    (ConcretisedNodeRepr::Star, term) => {
      match term {
        ConcretisedNodeRepr::App { root, arguments, origination } => {
          match origination {
            Origin::GlobalScope => {
              // check against signature
              let decl =
                declarations_table.retrieve_ref(&root).unwrap();
              if let DeclKind::WellScopedMapping { given_type: type_, .. } = decl.repr {
                let fun_type = unsafe { *type_ };
                examine_signature(
                  fun_type, arguments, diagnostic_delegate)
              } else {
                panic!("Havent found the object but should've had!")
              }
            },
            Origin::PatternBinding => todo!(),
            Origin::ContextBinding => todo!(),
          }

        },
        ConcretisedNodeRepr::Either(lt, rt) |
        ConcretisedNodeRepr::Pair(lt, rt) => {
          let lt = unsafe { *lt };
          check_static_shape(type_expr, lt, diagnostic_delegate, declarations_table);

          let rt = unsafe { *rt };
          check_static_shape(type_expr, rt, diagnostic_delegate, declarations_table);
        },
        ConcretisedNodeRepr::Void |
        ConcretisedNodeRepr::Singleton => {
          // this is always ok
        },
        ConcretisedNodeRepr::Sigma { head, spine } |
        ConcretisedNodeRepr::Arrow { head, spine , .. } => {
          let spine = unsafe { *spine };
          check_static_shape(type_expr, spine, diagnostic_delegate, declarations_table);

          // also check head
        },
        ConcretisedNodeRepr::Reference { name: ref_, origination } => {

        },
        _ => {
          // invalid combination of type and term
          let problem = ProblemReport {
            kind: Kind::MismatchedType {
              type_expr: type_expr.location,
              term_expr: term_expr.location
            }
          };
          diagnostic_delegate.report_problem(problem)
        }
      }
    },
    (ConcretisedNodeRepr::Singleton, ConcretisedNodeRepr::Pt) => {

    },
    (ConcretisedNodeRepr::Pair(lt, rt), ConcretisedNodeRepr::Tuple(lv, rv)) => {
      let lt = unsafe { *lt };
      let rt = unsafe { *rt };
      let lv = unsafe { *lv };
      let rv = unsafe { *rv };
      check_static_shape(lt, lv, diagnostic_delegate, declarations_table);
      check_static_shape(rt, rv, diagnostic_delegate, declarations_table);
    },
    (ConcretisedNodeRepr::Either(lt, _), ConcretisedNodeRepr::Left(lv)) => {
      let lt = unsafe { *lt };
      let lv = unsafe { *lv };
      check_static_shape(lt, lv, diagnostic_delegate, declarations_table);
    },
    (ConcretisedNodeRepr::Either(_, rt), ConcretisedNodeRepr::Right(rv)) => {
      let rt = unsafe { *rt };
      let rv = unsafe { *rv };
      check_static_shape(rt, rv, diagnostic_delegate, declarations_table);
    },
    (ConcretisedNodeRepr::Arrow { head, spine , .. },
      ConcretisedNodeRepr::Lam { rewrite_rules }) => {
      // lambda may be pi
    },
    (ConcretisedNodeRepr::Sigma { head, spine },
      ConcretisedNodeRepr::Wit { premises, conclusion }) => {

    },
    _ => {
      // invalid combination of type and term
      let problem = ProblemReport {
        kind: Kind::MismatchedType {
          type_expr: type_expr.location,
          term_expr: term_expr.location
        }
      };
      diagnostic_delegate.report_problem(problem)
    }
  }

}

fn examine_signature(
  fun_type: ConcretisedNode,
  input: ArrayPtr<ConcretisedNode>,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate
) {
  let ConcretisedNode {
    kind,
    location,
    implicit_context
  } = fun_type;

  if let ConcretisedNodeRepr::Arrow { head, spine , .. } = kind {

    let input_count = input.project_count();
    assert!(input_count == head.project_count());

    let mut ctx =
      HashMap::<Symbol, Option<ConcretisedNode>>::new();

    let head_ptr = head.project_ptr();
    let input_ptr = input.project_ptr();
    for i in 0 .. input_count {
      let (symbol, _) = unsafe { *head_ptr.add(i) };
      if let Some(symbol) = symbol {
        let input = unsafe { *input_ptr };
        let _ = ctx.insert(symbol, Some(input));
      }
    }


  } else {
    panic!("Unexpectedly received nonfunction type")
  }
}