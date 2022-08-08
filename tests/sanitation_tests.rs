use std::collections::HashSet;

use proto_sigil::{
  parser::new_parser::ParsingState,
  elaborator::{diagnostics::{ProblemReport, SomeDiagnosticsDelegate}, scope_analysis::concretise_declaration, presense_tester::PresenseSet, context_use_check::check_context_use, rewrite_system_check::check_rewrite_system}, expression_trees::better_nodes::DeclKind,
};

#[derive(Debug)]
struct FakeDiagDel {
  items: Vec<ProblemReport>
}
impl SomeDiagnosticsDelegate for FakeDiagDel {
  fn report_problem(&mut self, report: ProblemReport) {
    self.items.push(report)
  }
}

#[test]
fn test_smth_simple () {
  let example_text =
  "test_func : {T} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      //println!("{:?}", dd);
      assert!(dd.items.len() == 0);
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}


#[test]
fn duplicated_imp_ctx () {
  let example_text =
  "test_func : {T, T} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      //println!("{:?}", dd);
      assert!(dd.items.len() != 0);
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn vis_propogation () {
  let example_text =
  "test_func : {T} (T) -> {T} T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      //println!("{:?}", dd);
      assert!(dd.items.len() != 0);
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn vis_propogation_in_ctx () {
  let example_text =
  "test_func : {T: K, K: T} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      // println!("{:?}", dd);
      assert!(dd.items.len() == 0);
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn vis_in_ctx () {
  let example_text =
  "test_func : {T, K: d} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      // println!("{:?}", dd);
      assert!(dd.items.len() != 0);
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn use_check () {
  let example_text =
  "test_func : {T, K} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      if let DeclKind::WellScopedMapping {
        given_type: type_, ..
      } = decl.repr {
        let mut unused = HashSet::new();
        check_context_use(unsafe { *type_ }, &mut dd, &mut unused);
        //println!("{:#?}", unused);
        assert!(!unused.is_empty())
      } else {
        panic!()
      };
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }

  let example_text =
  "test_func : {T, K} (T) -> K\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      if let DeclKind::WellScopedMapping {
        given_type: type_, ..
      } = decl.repr {
        let mut unused = HashSet::new();
        check_context_use(unsafe { *type_ }, &mut dd, &mut unused);
        //println!("{:#?}", unused);
        assert!(unused.is_empty())
      } else {
        panic!()
      };
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn position_check () {
  let example_text =
  "test_func : {T, K} (T) -> T\n".to_string() +
  "| v => v" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      if let DeclKind::WellScopedMapping {
        given_type: type_, ..
      } = decl.repr {
        let mut unused = HashSet::new();
        check_context_use(unsafe { *type_ }, &mut dd, &mut unused);
        //println!("{:#?}", unused);
        assert!(!unused.is_empty())
      } else {
        panic!()
      };


    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn def_to_fun_conversion () {
  let example_text =
  "test_func : {T} (T) -> T =\n".to_string() +
  " \\{ | v => v }" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      if let DeclKind::WellScopedMapping {
        given_type: type_, ..
      } = decl.repr {
        let mut unused = HashSet::new();
        check_context_use(unsafe { *type_ }, &mut dd, &mut unused);
        //println!("{:#?}", unused);
        assert!(unused.len() == 0);
      } else {
        panic!()
      };

    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}

#[test]
fn rs_check () {
  let example_text =
  "test_func : (Either Dot Dot) -> Dot".to_string() +
  "| inr pt => pt\n" +
  "| inl pt => pt" ;

  let mut parser = ParsingState::init(&example_text);
  let decl = parser.parse_decl();
  match decl {
    Ok(decl) => {
      let mut decl = decl;
      let mut dd = FakeDiagDel { items: Vec::new() };
      let gs = PresenseSet::init();

      concretise_declaration(
        &mut decl, &mut dd,
        &gs);

      check_rewrite_system(decl, &mut dd);

        println!("{:#?}", dd)
    },
    Err(err) => {
      panic!("{:?}", err)
    },
  }
}