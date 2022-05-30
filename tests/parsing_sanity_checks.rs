use proto_sigil::{parser::{
  parser::{
    ParsingState, symbol::{Symbol, Repr}}},
  trees::{raw_syntax_nodes::{
    RefNode,  RawKind, Mapping},
    naive_textual_rendering::{render_expr_tree, render_pattern}}};

extern crate proto_sigil;



#[test]
fn recognisible_long() {
  let example_text = "aaaaaaaa";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth = ps.parse_symbol();
  //println!("{:#?}", smth);
  match smth {
    Result::Ok(Symbol { repr:
      Repr::OffsetInfo {
        offset_from_start: 0, offset_from_head: 8 } }) => (),
    _ => panic!("Expected a big symbol, but found something else")
  }
}

#[test]
fn recognisible_short () {
  let example_text = "aaaaaaa";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth = ps.parse_symbol();
  //println!("{:#?}", smth);
  match smth {
    Result::Ok(Symbol { repr:
      Repr::Inlined([0x61,0x61,0x61,0x61,0x61,0x61,0x61]) }) => (),
    _ => panic!("Expected small symbol of all 'a's, but found something else")
  }
}

#[test]
fn prefix_check_works() {
  let example_text = "aab";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());

  let matches = ps.prefix_match(
    "aab", false);
  assert!(matches == true);
  assert!(ps.byte_index == 0);

  let dont_match = ps.prefix_match(
    "aaa", false);
  assert!(dont_match == false);
  assert!(ps.byte_index == 0);
}

#[test]
fn depth_probing_works() {
  let example_text = "   \n  ";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let found_depth = ps.probe_depth();
  //println!("{}", found_depth);
  assert!(found_depth == 2);
}

#[test]
fn trivial_expr() {
  let example_text = "A";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let whatever =
    ps.parse_expr(0);
  println!("{:#?}", whatever);
  if let Ok(val) = whatever {
    let val = unsafe { *val.project_ptr().cast::<RefNode>() };
    println!("{:#?}", val);
  }
}

#[test]
fn basic_expr() {
  let example_text = "A (B C) \n D (E F G)";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let app_expr =
    ps.parse_expr(0);
  //println!("{:#?}", app_expr);
  match app_expr {
    Ok(expr_ptr) => {
      let kind = expr_ptr.project_tag();
      //println!("{:#?}", kind);
      assert!(kind == RawKind::App_ArgsInline);
      assert!(kind == RawKind::App_ArgsInline);
      let arg_count = expr_ptr.project_count();
      //println!("{}", arg_count);
      assert!(arg_count == 3);
      let mut thing = String::new();
      render_expr_tree(expr_ptr, &mut thing);
      //println!("{}", thing);
      assert!(thing == "(A [(B [C]) D (E [F G])])");
    },
    Err(err) => {
      println!("{:#?}", err);
      panic!("Unexpected failure");
    },
  }
}

#[test]
fn basic_pattern_expr() {
  let example_text = "A (B _ _) (C _ (D E))";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let pattern =
    ps.parse_pattern().unwrap();
  let mut str = String::new();
  render_pattern(pattern, &mut str);
  println!("{}", str);
}


#[test]
fn clause () {
  let example_text =
    "| A (B _ _), C _ (D E) => A (B C) D (E F G)";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let clause =
    ps.parse_clause(0);
  println!("{:#?}", clause);
}

#[test]
fn lambda () {
  let example_text =
    "\\{\n".to_string() +
    "  | A (B _ _), C _ (D E) => A (B C) \n   D (E F G)\n" +
    "| A (B _ _), C _ (D E) => A (B C) \n  D (E F G) }";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let clause =
    ps.parse_lambda();
  println!("{:#?}", clause);
}


#[test]
fn decl_parsing () {
  let example_text =
    "example : Either A B = \\{\n".to_string() +
    "  | A (B _ _), C _ (D E) => A (B C) \n   D (E F G)\n" +
    "| A (B _ _), C _ (D E) => A (B C) \n  D (E F G) }";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let def = ps.parse_decl().unwrap();
  println!("{:#?}", def.project_tag());
}


#[test]
fn map_parsing () {
  let example_text =
    "example : Either A B \n".to_string() +
    "  | A (B _ _), C _ (D E) => A (B C) \n   D (E F G)\n" +
    "| A (B _ _), C _ (D E) => A (B C) \n  D (E F G) }";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let def = ps.parse_decl().unwrap();
  let val = unsafe { &*def.project_ptr().cast::<Mapping>() };
  println!("{:#?}", val);
  let mut thing = String::new();
  render_expr_tree(val.type_, &mut thing);
  println!("{:#?}", thing);
}

#[test]
fn fun_parsing () {
  let example_text =
    "(a : T, K a a) -> (b : D) -> M a b";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth = ps.parse_lift_node();
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str);
      println!("{}", str);
    },
    Err(oops) => println!("{:#?}", oops),
  }
}

