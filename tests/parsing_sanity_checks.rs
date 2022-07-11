use proto_sigil::{parser::{
  parser::{
    ParsingState,}},
  expression_trees::{raw_syntax_nodes::{
    RefNode,  Mapping, RawCtxPtr},
    naive_textual_rendering::{render_expr_tree, render_pattern}}};

extern crate proto_sigil;





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
      let mut thing = String::new();
      render_expr_tree(expr_ptr, &mut thing, example_text);
      //println!("{}", thing);
      assert!(thing == "(A [(B [C]), D, (E [F, G])])");
    },
    Err(err) => {
      panic!("{:#?}", err);
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
  render_pattern(pattern, &mut str, example_text);
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
  render_expr_tree(val.type_, &mut thing, &example_text);
  println!("{:#?}", thing);
}

#[test]
fn fun_parsing () {
  let example_text =
    "(a : T, K a a) -> (b : D) -> M a b";
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth =
    ps.parse_lift_node(RawCtxPtr::init_null());
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str, example_text);
      println!("{}", str);
      assert!(str == "(a : T, (K [a, a])) -> ((b : D) -> ((M [a, b])))")
    },
    Err(oops) => panic!("{:#?}", oops),
  }
}

#[test]
fn some_realistic_def() {
  let example_text =
    "example : (a : Dot) -> Dot\n".to_string() +
    "| pt => pt" ;
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth = ps.parse_decl();
  match smth {
    Ok(val) => {
      println!("{:#?}", val);
    },
    Err(err) => {
      panic!("{:#?}", err);
    }
  }
}

#[test]
fn implicits_are_parseable () {
  let example_text =
    "{A, B, C : M n} (a : A) -> B" ;
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth =
    ps.parse_expr(0);
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str, example_text);
      println!("{}", str);
      assert!(str == "{A, B, C : (M [n])} (a : A) -> (B)")
    },
    Err(err) => {
      panic!("{:#?}", err);
    }
  }
}

#[test]
fn witness_is_parseable () {
  let example_text =
    "[| a b , b c ; c d (e f) |]" ;
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth =
    ps.parse_expr(0);
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str, example_text);
      println!("{}", str);
      assert!(str == "[| (a [b]), (b [c]) ; (c [d, (e [f])]) |]")
    },
    Err(err) => {
      panic!("{:#?}", err);
    }
  }
}

#[test]
fn ref_node_with_ctx_is_parseable () {
  let example_text =
    "{A} A" ;
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth =
    ps.parse_expr(0);
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str, example_text);
      println!("{}", str);
      assert!(str == "{A} A")
    },
    Err(err) => {
      panic!("{:#?}", err);
    }
  }
}

#[test]
fn nested_parens() {
  let example_text =
    "(((a : A) -> T a))" ;
  let mut ps =
    ParsingState::init(
      example_text.as_bytes());
  let smth =
    ps.parse_expr(0);
  match smth {
    Ok(val) => {
      let mut str = String::new();
      render_expr_tree(val, &mut str, example_text);
      println!("{:#?}", str);
      assert!(str == "(a : A) -> ((T [a]))")
    },
    Err(err) => {
      panic!("{:#?}", err);
    }
  }
}