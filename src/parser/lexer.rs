
use super::parser::symbol::Symbol;



pub enum LexingItemRepr {
  Name(Symbol), Eq, Indent(u8), Qm, Ln, Expr
}
pub struct LexingItem {
  pub repr: LexingItemRepr,
}
pub enum DefKind {
  Mapping, Declaration
}

pub struct Lexer<'a> {
  pub bytes_ptr: &'a [u8],
  pub byte_ptr: u32,
  pub single_derivation: Vec<LexingItem>,
}

impl Lexer<'_> {
  
}