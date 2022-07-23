use std::{collections::HashSet, borrow::Cow,};

use crate::{
  expression_trees::{
    better_nodes::{
      Declaration, DeclKind, RawNode,
      RawNodeRepr, ConcretisedNode, ConcretisedNodeRepr, Symbol, RawRewriteRule,
      RawPattern, ConcretisedPattern, ConcretisedRewriteRule, RawPatternKind,
      ConcretisedPatternKind, Origin,} },};

use super::{
  diagnostics::{
    ProblemReport, Kind, SomeDiagnosticsDelegate
  },
  presense_tester::PresenseSet
};



pub fn concretise_declaration(
  given_decl: &mut Declaration,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  global_symbols: &PresenseSet<Symbol>
) {
  match given_decl.repr {
    DeclKind::RawMapping { name, given_type: type_, rewrite_rules } => {
      concretise_expr(
        type_,
        diagnostic_delegate,
        global_symbols,
        &HashSet::new(), &HashSet::new());
      let saned_type = type_.cast::<ConcretisedNode>();

      let ptr = rewrite_rules.project_ptr();
      let lim = rewrite_rules.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        concretise_rewrite_rule(
          ptr, diagnostic_delegate,
          global_symbols, &HashSet::new(), &HashSet::new())
      }
      let checked_rrs =
        rewrite_rules.cast::<ConcretisedRewriteRule>();

      let saned_map = DeclKind::WellScopedMapping {
        name, given_type: saned_type, rewrite_rules: checked_rrs
      };
      given_decl.repr = saned_map
    },
    DeclKind::RawDefinition { name, given_type: type_, value } => {

      // promote definition of function to function declaration.
      // for sake of simplifying logic in some places.
      if let RawNodeRepr::Fun { .. } = unsafe { *type_ }.kind {
      if let RawNodeRepr::Lam { rewrite_rules: rrs } = unsafe { *value }.kind {
        let promoted_decl = DeclKind::RawMapping {
          name, given_type: type_, rewrite_rules: rrs
        };
        given_decl.repr = promoted_decl;

        concretise_declaration(
          given_decl, diagnostic_delegate, global_symbols);
        return;
      } }

      concretise_expr(
        type_, diagnostic_delegate,
        global_symbols, &HashSet::new(), &HashSet::new());
      concretise_expr(
        value, diagnostic_delegate,
        global_symbols, &HashSet::new(), &HashSet::new());
      let saned_type = type_.cast::<ConcretisedNode>();
      let saned_value = value.cast::<ConcretisedNode>();
      let saned_def =
        DeclKind::WellScopedDefinition { name, given_type: saned_type, value: saned_value };
      given_decl.repr = saned_def;
    },
    _ => panic!("No need to sanitise things twice")
  }
}

fn concretise_expr(
  expr_ptr: *mut RawNode,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  global_symbols: &PresenseSet<Symbol>,
  context_symbols: &HashSet<Symbol>,
  pattern_binders: &HashSet<Symbol>
) {
  let RawNode {
    kind,
    location,
    implicit_context
  } = unsafe { *expr_ptr };

  let mut context_symbols = Cow::Borrowed(context_symbols);
  let mut duplicated_imp_ctx_items = HashSet::new();
  let san_ctx;

  if let Some(ctx) = implicit_context {
    let ptr = ctx.project_ptr();
    let lim = ctx.project_count();

    for i in 0 .. lim {
      let (symbol, _) = unsafe { *ptr.add(i) };
      if context_symbols.contains(&symbol) {
        duplicated_imp_ctx_items.insert(symbol);
      } else {
        context_symbols.to_mut().insert(symbol);
      }
    }
    if !duplicated_imp_ctx_items.is_empty() {
      let problem = ProblemReport {
        kind: Kind::DuplicatesInImpCtx(duplicated_imp_ctx_items)
      };
      diagnostic_delegate.report_problem(problem)
    }
    for i in 0 .. lim {
      let (_, expr) = unsafe { &mut *ptr.add(i) };
      if let Some(expr) = expr {
        concretise_expr(
          expr, diagnostic_delegate,
          global_symbols, context_symbols.as_ref(), pattern_binders)
      }
    }
    let ctx =
      ctx.cast::<(Symbol, Option<ConcretisedNode>)>();
    san_ctx = Some(ctx)
  } else {
    san_ctx = None
  }

  let context_symbols = context_symbols.as_ref();

  let checked_kind: ConcretisedNodeRepr ;
  match kind {
    RawNodeRepr::Star => {
      checked_kind = ConcretisedNodeRepr::Star
    },
    RawNodeRepr::Ref(symbol) => {
      let str = symbol.materialise_name();
      match str {
        "Void" => {
          checked_kind = ConcretisedNodeRepr::Void ;
        },
        "Dot" => {
          checked_kind = ConcretisedNodeRepr::Singleton ;
        },
        "pt" => {
          checked_kind = ConcretisedNodeRepr::Pt
        },
        _ if pattern_binders.contains(&symbol) => {
          checked_kind = ConcretisedNodeRepr::Reference {
            ref_: symbol, origination: Origin::PatternBinding
          }
        },
        _ if context_symbols.contains(&symbol) => {
          checked_kind = ConcretisedNodeRepr::Reference {
            ref_: symbol, origination: Origin::ContextBinding
          }
        },
        _ if global_symbols.check_out(&symbol) => {
          checked_kind = ConcretisedNodeRepr::Reference {
            origination: Origin::GlobalScope,
            ref_: symbol
          }
        },
        _ => {
          let problem = ProblemReport {
            kind: Kind::IrrelevantSymbol(symbol)
          };
          diagnostic_delegate.report_problem(problem);
          return ()
        }
      }
    },
    RawNodeRepr::App { root, arguments } => {

      let ptr = arguments.project_ptr();
      let lim = arguments.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        concretise_expr(
          ptr, diagnostic_delegate,
          global_symbols, context_symbols, pattern_binders)
      }
      let checked_args =
        arguments.cast::<ConcretisedNode>();

      let origination: Origin;
      match () {
        _ if pattern_binders.contains(&root) => {
          origination = Origin::PatternBinding
        },
        _ if context_symbols.contains(&root) => {
          origination = Origin::ContextBinding
        },
        _ if global_symbols.check_out(&root) => {
          origination = Origin::GlobalScope;
        },
        _ => {
          let problem = ProblemReport {
            kind: Kind::IrrelevantSymbol(root)
          };
          diagnostic_delegate.report_problem(problem);
          return
        }
      }
      checked_kind = ConcretisedNodeRepr::App {
        root, arguments: checked_args, origination
      };
    },
    RawNodeRepr::Wit { premises, conclusion } => {
      let ptr = premises.project_ptr();
      let lim = premises.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        concretise_expr(
          ptr, diagnostic_delegate,
          global_symbols, context_symbols, pattern_binders)
      }
      let checked_prems =
        premises.cast::<ConcretisedNode>();

      concretise_expr(
        conclusion, diagnostic_delegate,
        global_symbols, context_symbols, pattern_binders);
      let checked_conc = conclusion.cast::<ConcretisedNode>();
      checked_kind = ConcretisedNodeRepr::Wit {
        premises: checked_prems, conclusion: checked_conc };
    },
    RawNodeRepr::Fun { head, spine } |
    RawNodeRepr::Sigma { head, spine } => {
      let ptr = head.project_ptr();
      let lim = head.project_count();

      let mut head_names = context_symbols.clone();
      let mut duplicated_binders = HashSet::new();
      let mut does_perform_introspection = false;
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        let (name, _) = unsafe { *ptr };
        if let Some(name) = name {
          let new = head_names.insert(name);
          if !new {
            duplicated_binders.insert(name);
          }
          does_perform_introspection = true
        }
      }
      if !duplicated_binders.is_empty() {
        let problem = ProblemReport {
          kind: Kind::DuplicatedBinders(duplicated_binders)
        };
        diagnostic_delegate.report_problem(problem)
      }

      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        let (_, node) = unsafe { &mut *ptr };
        concretise_expr(
          node, diagnostic_delegate,
          global_symbols, &mut head_names, pattern_binders)
      }
      let checked_head =
        head.cast::<(Option<Symbol>, ConcretisedNode)>();

      concretise_expr(
        spine, diagnostic_delegate,
        global_symbols, &mut head_names, pattern_binders);
      let checked_spine = spine.cast::<ConcretisedNode>();

      checked_kind = match kind {
        RawNodeRepr::Fun { .. } => {
          ConcretisedNodeRepr::Arrow {
            head: checked_head, spine: checked_spine,
            performs_introspection: does_perform_introspection
          }
        },
        RawNodeRepr::Sigma { .. } => {
          ConcretisedNodeRepr::Sigma { head: checked_head, spine: checked_spine }
        },
        _ => unreachable!()
      }
    },
    RawNodeRepr::Lam { rewrite_rules } => {
      let ptr = rewrite_rules.project_ptr();
      let lim = rewrite_rules.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        concretise_rewrite_rule(
          ptr, diagnostic_delegate, global_symbols, context_symbols, pattern_binders)
      }
      let checked_rrs =
        rewrite_rules.cast::<ConcretisedRewriteRule>();
      checked_kind = ConcretisedNodeRepr::Lam { rewrite_rules: checked_rrs }
    },
  };
  let checked_node = ConcretisedNode {
    implicit_context: san_ctx,
    kind: checked_kind,
    location,
  };

  unsafe { *expr_ptr.cast::<ConcretisedNode>() = checked_node }

}

fn concretise_rewrite_rule(
  rule: *mut RawRewriteRule,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  global_symbols: &PresenseSet<Symbol>,
  context_symbols: &HashSet<Symbol>,
  pattern_binders: &HashSet<Symbol>,
) {
  let RawRewriteRule { matchers, lhs , location }
    = unsafe { *rule };

  let mut rule_local_binders = pattern_binders.clone();
  let mut duplicated_binders = HashSet::new();
  let ptr = matchers.project_ptr();
  let count = matchers.project_count();
  for i in 0 .. count {
    let ptr = unsafe { ptr.add(i) };
    concretise_pattern(
      ptr, diagnostic_delegate,
      &mut rule_local_binders, &mut duplicated_binders)
  }
  if !duplicated_binders.is_empty() {
    let problem = ProblemReport {
      kind: Kind::DuplicatedBinders(duplicated_binders)
    };
    diagnostic_delegate.report_problem(problem)
  }
  let checked_matchers =
    matchers.cast::<ConcretisedPattern>();

  concretise_expr(
    lhs, diagnostic_delegate,
    global_symbols, context_symbols, &rule_local_binders);
  let checked_lhs = lhs.cast::<ConcretisedNode>();

  let checked_rule = ConcretisedRewriteRule {
    matchers: checked_matchers,
    rhs: checked_lhs,
    location
  };

  unsafe { *rule.cast() = checked_rule }
}

fn concretise_pattern(
  pattern: *mut RawPattern,
  diagnostic_delegate: &mut dyn SomeDiagnosticsDelegate,
  local_symbols: &mut HashSet<Symbol>,
  duplicated_binders: &mut HashSet<Symbol>,
) {
  let RawPattern { repr, location }
    = unsafe { *pattern };

  let checked_repr;
  match repr {
    RawPatternKind::Wildcard => {
      checked_repr = ConcretisedPatternKind::Wildcard
    },
    RawPatternKind::Compound { head, subexpressions } => {

      let ptr = subexpressions.project_ptr();
      let lim = subexpressions.project_count();
      for i in 0 .. lim {
        let ptr = unsafe { ptr.add(i) };
        concretise_pattern(
          ptr, diagnostic_delegate,
          local_symbols, duplicated_binders);
      }

      let ref_ = head.materialise_name();
      match ref_ {
        "two" => {
          if lim != 2 {
            let problem = ProblemReport {
              kind: Kind::IncorrectArity(location)
            };
            diagnostic_delegate.report_problem(problem);
            return
          }
          let l = ptr.cast::<ConcretisedPattern>();
          let r = unsafe { ptr.add(1) }.cast::<ConcretisedPattern>();
          checked_repr = ConcretisedPatternKind::Tuple(l, r);
        },
        "inl" => {
          if lim != 1 {
            let problem = ProblemReport {
              kind: Kind::IncorrectArity(location)
            };
            diagnostic_delegate.report_problem(problem);
            return
          }
          let l = ptr;
          checked_repr = ConcretisedPatternKind::Left(l.cast());
        },
        "inr" => {
          if lim != 1 {
            let problem = ProblemReport {
              kind: Kind::IncorrectArity(location)
            };
            diagnostic_delegate.report_problem(problem);
            return
          }
          let r = ptr;
          checked_repr = ConcretisedPatternKind::Right(r.cast());
        },
        _ => {
          let problem = ProblemReport {
            kind: Kind::InvalidDeconstructionPattern(head)
          };
          diagnostic_delegate.report_problem(problem);
          return
        }
      }
    },
    RawPatternKind::Mono(symbol) => {
      let ref_ = symbol.materialise_name();
      match ref_ {
        "pt" => {
          checked_repr = ConcretisedPatternKind::Pt
        },
        _ => {
          let new = local_symbols.insert(symbol);
          if !new {
            duplicated_binders.insert(symbol);
            return
          } else {
            checked_repr = ConcretisedPatternKind::VarBinding(symbol)
          }
        }
      }
    },
  }
  let checked_pattern = ConcretisedPattern {
    location,
    repr: checked_repr
  };

  unsafe { *pattern.cast() = checked_pattern }
}