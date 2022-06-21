
use proto_sigil::{support_structures::no_bullshit_closure::{
  Closure,},
  closure,
  build_arg_destructor_tuple,
  build_capture_tuple,
  build_destructor_tuple,
  mk_ty_rec};


#[test]
fn clos_works() {
  struct Ctx { str: String }
  let str = "ho".to_string();
  let ctx = Ctx { str };
  let clos =
  Closure::<Ctx, (), _>::init_with_global_mem(
  ctx, |env, _| {
    let env = unsafe { env.read() };
    println!("{}", env.str)
  });

  clos.invoke_consume(());
}

#[test]
fn clos_works2 () {
  let str = "yo".to_string();
  let clos = closure!([str] {
    assert!("yo" == str)
  });
  clos.invoke_consume(());
}

#[test]
fn clos_works3 () {
  let clos = closure!([] {
    assert!(true)
  });
  clos.invoke_consume(());
}

#[test]
fn clos_works4 () {
  let clos = closure!([] {
    100
  });
  let thing = clos.erase_env_type().try_invoke(()).unwrap();
  assert!(thing == 100)
}

#[test]
fn clos_works5 () {
  let mut str = "yo".to_string();
  let clos = closure!([str = &mut str] {
    str.push_str(" sup?")
  });
  let () = clos.erase_env_type().try_invoke(()).unwrap();
  assert!(str == "yo sup?")

}