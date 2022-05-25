
use crate::parser::{
  parser::symbol::{Repr, Symbol},
  node_allocator::EntangledPtr};
use super::raw_syntax_nodes::{
  RawKind, RefNode, AppNodeIndirectSmall, ExprPtr, AppNodeArgsInline};


pub fn render_expr_tree(expr: ExprPtr, output: &mut String) { unsafe {
  let kind = expr.project_tag();
  let ptr = expr.project_ptr();
  match kind {
    RawKind::Ref => {
      let ref_node = *ptr.cast::<RefNode>();
      write_symbol(ref_node.name, output);
    },
    RawKind::App_ArgsInSlab => {
      let app_node = *ptr.cast::<AppNodeIndirectSmall>();
      write_symbol(app_node.name, output);
      output.push_str(" [");
      let count = expr.project_count();
      let ptr = app_node.args.reach_referent_from(ptr);
      let limit = count - 1;
      for i in 0 .. count {
        let arg =
          (*ptr.cast::<EntangledPtr>().add(i as usize))
          .reach_referent_from(ptr).cast::<ExprPtr>();
        render_expr_tree(*arg, output);
        if i != limit {
          output.push(' ');
        }
      }
      output.push_str("]");
    },
    RawKind::App_ArgsInline => {
      output.push('(');
      let app_node = *ptr.cast::<AppNodeArgsInline>();
      write_symbol(app_node.name, output);
      output.push_str(" [");
      let count = expr.project_count();
      let ptr = app_node.args;
      let limit = count - 1;
      for i in 0 .. count {
        let arg = ptr.get_unchecked(i as usize);
        render_expr_tree(*arg, output);
        if i != limit {
          output.push(' ');
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
    RawKind::Wit => todo!(),
    RawKind::Fun => todo!(),
    RawKind::Sig => todo!(),
    RawKind::Star => {
      output.push('*');
    },
  }
} }

fn write_symbol(symbol: Symbol, output: &mut String) {
  match symbol.repr {
    Repr::Inlined(smth) => {
      for char in smth.iter() {
        let char = *char;
        if char == 0 { break; }
        let char = unsafe {
          char::from_u32_unchecked(char as u32) };
        output.push(char);
      }
    },
    Repr::OffsetInfo {
      offset_from_start,
      offset_from_head
    } => {
      todo!()
    },
  }
}