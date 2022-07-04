
use crate::parser::{
  parser::symbol::{Symbol}};
use super::raw_syntax_nodes::{
  RawKind, RefNode, AppNodeIndirectSmall, ExprPtr, AppNodeArgsInline,
  PatternExprPtr, PatternKind, CompoundPatternNode_ArgsInline,
  RefPatternNode, LiftNode, LiftNodeItem, RawCtxPtr, WitnessNodeIndirect};


pub fn render_expr_tree(expr: ExprPtr, output: &mut String, str: &str) { unsafe {
  let kind = expr.project_presan_tag();
  let ptr = expr.project_ptr();
  match kind {
    RawKind::Ref => {
      let ref_node = *ptr.cast::<RefNode>();
      write_implicit_context_if_present(ref_node.ctx_ptr, output, str);
      write_symbol(&ref_node.name, output, str);
    },
    RawKind::App_ArgsInSlab => {
      let app_node = *ptr.cast::<AppNodeIndirectSmall>();
      write_implicit_context_if_present(app_node.ctx_ptr, output, str);
      output.push('(');
      write_symbol(&app_node.name, output, str);
      output.push_str(" [");
      let count = expr.project_count();
      let args =
        app_node.args.reach_referent_from(ptr).cast::<ExprPtr>();
      let limit = count - 1;
      for i in 0 .. count {
        let arg = args.add(i as usize);
        render_expr_tree(*arg, output, str);
        if i != limit {
          output.push_str(", ");
        }
      }
      output.push_str("])");
    },
    RawKind::App_ArgsInline => {
      let app_node = *ptr.cast::<AppNodeArgsInline>();
      write_implicit_context_if_present(app_node.ctx_ptr, output, str);
      output.push('(');
      write_symbol(&app_node.name, output, str);
      output.push_str(" [");
      let count = expr.project_count();
      let args = &app_node.args;
      let limit = count - 1;
      for i in 0 .. count {
        let arg = args.get_unchecked(i as usize);
        render_expr_tree(*arg, output, str);
        if i != limit {
          output.push_str(", ");
        }
      }
      output.push_str("])");
    },
    RawKind::App_ArgsInVec => {
      todo!()
    },
    RawKind::Lam => {
      let repr = "\\{ ... }";
      output.push_str(repr);
    },
    RawKind::Wit => {
      output.push_str("[| ");
      let count = expr.project_count();
      let limit = count - 1;
      let data =
        &mut *ptr.cast::<WitnessNodeIndirect>();
      let args =
        data.items.reach_referent_from(ptr).cast::<ExprPtr>();
      for i in 0 .. count {
        let arg = *args.add(i as usize);
        render_expr_tree(arg, output, str);
        if i != limit { output.push_str(", ") }
      }
      output.push_str(" ; ");
      render_expr_tree(data.seal, output, str);
      output.push_str(" |]");
    },
    RawKind::Fun => {
      let node = &*ptr.cast::<LiftNode>();
      write_implicit_context_if_present(node.ctx_ptr, output, str);
      output.push('(');
      let count = expr.project_count();
      let items =
        node.head.reach_referent_from(ptr).cast::<LiftNodeItem>();
      let limit = count - 1;
      for i in 0 .. count {
        let item = &*items.add(i as usize);
        if let Some(ref name) = item.name {
          write_symbol(name, output, str);
          output.push_str(" : ");
        }
        render_expr_tree(item.val, output, str);
        if i != limit { output.push_str(", ")};
      }
      output.push_str(") -> ");
      output.push('(');
      render_expr_tree(node.spine_expr, output, str);
      output.push(')');
    },
    RawKind::Sig => {
      let node = &*ptr.cast::<LiftNode>();
      write_implicit_context_if_present(node.ctx_ptr, output, str);
      output.push('(');
      let count = expr.project_count();
      let items =
        node.head.reach_referent_from(ptr).cast::<LiftNodeItem>();
      let limit = count - 1;
      for i in 0 .. count {
        let item = &*items.add(i as usize);
        if let Some(ref name) = item.name {
          write_symbol(name, output, str);
          output.push_str(" : ");
        }
        render_expr_tree(item.val, output, str);
        if i != limit { output.push_str(", ")};
      }
      output.push_str(") |- ");
      output.push('(');
      render_expr_tree(node.spine_expr, output, str);
      output.push(')');
    },
    RawKind::Star => {
      output.push('*');
    },
  }
} }

fn write_symbol(symbol: &Symbol, output: &mut String, str: &str) {
  let range =
    symbol.sloc.primary_offset as usize ..
    symbol.sloc.secondary_offset as usize;
  let str = &str[range];
  output.push_str(str);
}

pub fn render_pattern(
  node_ptr: PatternExprPtr, output: &mut String, str: &str
) { unsafe {
  let kind = node_ptr.project_tag();
  let ptr = node_ptr.project_ptr();
  match kind {
    PatternKind::Wildcard => {
      output.push('_');
    },
    PatternKind::Compound_Inlined => {
      output.push('(');
      let node =
        *ptr.cast::<CompoundPatternNode_ArgsInline>();
      write_symbol(&node.name, output, str);
      output.push_str(" [");
      let count = node_ptr.project_count();
      let limit = count - 1;
      for i in 0 .. count {
        let arg = node.args.get_unchecked(i as usize);
        render_pattern(*arg, output, str);
        if i != limit {
          output.push(' ');
        }
      }
      output.push_str("])");
    },
    PatternKind::Compound_Indirect => todo!(),
    PatternKind::Compound_Huge => todo!(),
    PatternKind::Singular => {
      let node = *ptr.cast::<RefPatternNode>();
      write_symbol(&node.name, output, str);
    },
  }
} }

fn write_implicit_context_if_present(
  ctx: RawCtxPtr, output: &mut String, str: &str) {
  if ctx.is_null() { return; }
  let ptr =
    ctx.project_ptr().cast::<(Symbol, ExprPtr)>();
  let count = ctx.project_count();
  let limit = count - 1;
  output.push('{');
  for i in 0 .. count {
    let (symbol, expr) =
      unsafe { *ptr.add(i as usize) };
    write_symbol(&symbol, output, str);
    if !expr.is_null() {
      output.push_str(" : ");
      render_expr_tree(expr, output, str);
    }
    if i != limit { output.push_str(", ") }
  }
  output.push_str("} ");
}