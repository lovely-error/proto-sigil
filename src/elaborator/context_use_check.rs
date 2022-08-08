


use std::collections::HashSet;

use crate::expression_trees::better_nodes::{
  ConcretisedNode, Symbol, ConcretisedNodeRepr, ConcretisedRewriteRule};
use super::diagnostics::{SomeDiagnosticsDelegate, ProblemReport, Kind};




pub fn check_context_use(
  node: ConcretisedNode,
  diagnostic_service: &mut dyn SomeDiagnosticsDelegate,
  encounted_items: &mut HashSet<Symbol>
) {
  let ConcretisedNode {
    implicit_context,
    kind,
    location
  } = node ;

  if let Some(ctx) = implicit_context {
    let ptr = ctx.project_ptr();
    let lim = ctx.project_count();
    for i in 0 .. lim {
      let (symbol, _) = unsafe { *ptr.add(i) };
      encounted_items.insert(symbol);
    }
    for i in 0 .. lim {
      let (_, expr) = unsafe { *ptr.add(i) };
      if let Some(expr) = expr {
        check_context_use(
          expr, diagnostic_service, encounted_items);
      }
    }
  }

  match kind {
    ConcretisedNodeRepr::Star |
    ConcretisedNodeRepr::Void |
    ConcretisedNodeRepr::Singleton |
    ConcretisedNodeRepr::Pt => {
      if let Some(_) = implicit_context {
        let problem = ProblemReport {
          kind: Kind::UnsedImpCtxAtTerminalNode(location)
        };
        diagnostic_service.report_problem(problem)
      }
    },
    ConcretisedNodeRepr::Reference { name: ref_, .. } => {
      encounted_items.remove(&ref_);
    },
    ConcretisedNodeRepr::App { root, arguments, .. } => {
      encounted_items.remove(&root);
      let ptr = arguments.project_ptr();
      let lim = arguments.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { *ptr.add(i) };
        check_context_use(
          ptr, diagnostic_service, encounted_items);
      }
    },
    ConcretisedNodeRepr::Wit { premises, conclusion } => {
      let ptr = premises.project_ptr();
      let lim = premises.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { *ptr.add(i) };
        check_context_use(
          ptr, diagnostic_service, encounted_items);
      }

      check_context_use(
        unsafe { *conclusion }, diagnostic_service, encounted_items);
    },
    ConcretisedNodeRepr::Sigma { head, spine } |
    ConcretisedNodeRepr::Arrow { head, spine, .. } => {
      let ptr = head.project_ptr();
      let lim = head.project_count();
      for i in 0 .. lim {
        let (_, expr) = unsafe { *ptr.add(i) };
        check_context_use(
          expr, diagnostic_service, encounted_items);
      }
      check_context_use(
        unsafe { *spine }, diagnostic_service, encounted_items);
    },
    ConcretisedNodeRepr::Lam { rewrite_rules } => {
      let ptr = rewrite_rules.project_ptr();
      let lim = rewrite_rules.project_count();
      for i in 0 .. lim {
        let ConcretisedRewriteRule { rhs: lhs, .. }
        = unsafe { *ptr.add(i) };
        check_context_use(
          unsafe { *lhs }, diagnostic_service, encounted_items);
      }
    },
    ConcretisedNodeRepr::Pair(l, r) |
    ConcretisedNodeRepr::Tuple(l, r) |
    ConcretisedNodeRepr::Either(l, r) => {
      let l = unsafe { *l };
      let r = unsafe { *r };
      check_context_use(
        l, diagnostic_service, encounted_items);
      check_context_use(
        r, diagnostic_service, encounted_items);
    },
    ConcretisedNodeRepr::Left(v) |
    ConcretisedNodeRepr::Right(v) => {
      check_context_use(
        unsafe { *v }, diagnostic_service, encounted_items);
    },
  }
}