use proto_sigil::{
  parser::new_parser::ParsingState,
  expression_trees::more_text_rendering::render_expr_tree};


#[test]
fn fun_parsing () {
  let example_text =
    "{T, K} (a : T, K a a) -> (b : D) -> M a b".to_string();
  let mut ps =
    ParsingState::init(
      &example_text);
  let smth =
    ps.parse_expr(0);
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str);
      println!("{}", str);
    },
    Err(oops) => panic!("{:#?}", oops),
  }
}


#[test]
fn basic_pattern_expr() {
  let example_text = "A (B _ _) (C _ (D E))".to_string();
  let mut ps =
    ParsingState::init(
      &example_text);
  let pattern =
    ps.parse_pattern().unwrap();
  println!("{:#?}", pattern)
}


#[test]
fn clause () {
  let example_text =
    "| A (B _ _), C _ (D E) => A (B C) D (E F G)".to_string();
  let mut ps =
    ParsingState::init(
      &example_text);
  let clause =
    ps.parse_clause(0);
  println!("{:#?}", clause);
}

//#[test]
fn sel_clause () {
  let example_text =
    "| A (B _ _), C _ (D E)\n".to_string() +
    "  ? true, true => A (B C) D E\n" +
    "  ? _ => C D (E A) B";

}