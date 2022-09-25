use crate::support_structures::{homemade_slice::Slice, raw_array_iter::RawArrayIter};

use super::{
  better_nodes::{
    RawNode, RawNodeRepr, Symbol
  },
  raw_syntax_nodes::SourceLocation
};



pub fn render_expr_tree(expr: RawNode, output: &mut String) {
  match expr.kind {
    RawNodeRepr::Star => {
      output.push('*')
    },
    RawNodeRepr::Ref(symbol) => {
      write_symbol(symbol, output)
    },
    RawNodeRepr::App { root, arguments } => {
      output.push_str("(");
      write_symbol(root, output);
      output.push_str(" [");
      for node in RawArrayIter::from_array_ptr(arguments) {
        render_expr_tree(node, output);
        output.push_str(", ");
      }
      output.push_str("])")
    },
    RawNodeRepr::Wit { premises, conclusion } => {
      output.push_str("[| ");
      for node in RawArrayIter::from_array_ptr(premises) {
        render_expr_tree(node, output);
        output.push(',');
      }
      output.push_str(" ; ");
      render_expr_tree(unsafe { *conclusion }, output);
      output.push_str(" |]");
    },
    RawNodeRepr::Fun { head, spine } => {
      output.push('(');
      for (name, expr) in RawArrayIter::from_array_ptr(head) {
        if let Some(symbol) = name {
          write_symbol(symbol, output);
          output.push_str(" : ");
        }
        render_expr_tree(expr, output);
        output.push_str(", ")
      }
      output.push_str(") -> ");
      render_expr_tree(unsafe { *spine }, output);
    },
    RawNodeRepr::Sigma { head, spine } => {
      output.push('(');
      for (name, expr) in RawArrayIter::from_array_ptr(head) {
        if let Some(symbol) = name {
          write_symbol(symbol, output);
          output.push_str(" : ");
        }
        render_expr_tree(expr, output);
        output.push_str(", ")
      }
      output.push_str(") |- ");
      render_expr_tree(unsafe { *spine }, output);
    },
    RawNodeRepr::Lam { rewrite_rules } => {
      output.push_str("\\{ ... }")
    },
  }
}

fn write_symbol(symbol: Symbol, output: &mut String) {
  let Slice { source_data, span } = symbol.chars_ptr;
  let slice = unsafe {
    std::slice::from_raw_parts(source_data, span as usize)
  };
  let SourceLocation { primary_offset, secondary_offset } = symbol.location;
  let slice = &slice[primary_offset as usize .. secondary_offset as usize ];
  let str = std::str::from_utf8(slice).unwrap();
  output.push_str(str);
}