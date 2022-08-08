use std::collections::HashSet;

use crate::expression_trees::{
  better_nodes::{
    Declaration, DeclKind, Symbol, ConcretisedNode, ConcretisedNodeRepr}};

use super::environment::PasteboardTable;



fn detect_cycles(
  definition: &Declaration,
  steps: &mut HashSet<Symbol>,
  global_scope: &PasteboardTable<Symbol, Declaration>,
  will_generate_ground_forms: &mut bool,
) {
  match definition.repr {
    DeclKind::WellScopedMapping { name, given_type, rewrite_rules } => {

    },
    DeclKind::WellScopedDefinition { name, given_type, value } => {
      steps.insert(name);

      trace_dependencies(
        steps,
        unsafe { &*value },
        will_generate_ground_forms,
        global_scope)
    },
    _ => {
      panic!("detect_cycles: not a well-scoped definition");
    }
  }
}

fn trace_dependencies(
  steps: &mut HashSet<Symbol>,
  expr: &ConcretisedNode,
  will_generate_ground_forms: &mut bool,
  global_scope: &PasteboardTable<Symbol, Declaration>,
) {
  let ConcretisedNode {
    kind,
    location,
    implicit_context
  } = expr;
  match kind {
    ConcretisedNodeRepr::Star => (),
    ConcretisedNodeRepr::Void |
    ConcretisedNodeRepr::Singleton |
    ConcretisedNodeRepr::Pt => {
      *will_generate_ground_forms = true;
    },
    ConcretisedNodeRepr::Reference { name, .. } => {
      if steps.contains(name) {
        //
      } else {
        let declaration =
          global_scope.retrieve_ref(name).unwrap();

        detect_cycles(
          &declaration,
          steps,
          global_scope,
          will_generate_ground_forms)
      }
    },
    ConcretisedNodeRepr::App { root, arguments, origination } => todo!(),
    ConcretisedNodeRepr::Wit { premises, conclusion } => todo!(),
    ConcretisedNodeRepr::Sigma { head, spine } => todo!(),
    ConcretisedNodeRepr::Arrow { head, spine, performs_introspection } => todo!(),
    ConcretisedNodeRepr::Lam { rewrite_rules } => todo!(),
    ConcretisedNodeRepr::Pair(_, _) => todo!(),
    ConcretisedNodeRepr::Tuple(_, _) => todo!(),
    ConcretisedNodeRepr::Either(_, _) => todo!(),
    ConcretisedNodeRepr::Left(_) => todo!(),
    ConcretisedNodeRepr::Right(_) => todo!(),
  }
}