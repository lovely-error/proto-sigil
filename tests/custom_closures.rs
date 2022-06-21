
use proto_sigil::{support_structures::no_bullshit_closure::{
  Closure, SupportTypeErasure}, closure};


//#[test]
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

  let erased = clos.erase_type();
  erased.invoke_consume(());
}

#[test]
fn clos_works2 () {
  let str = "yo".to_string();
  let clos = closure!([str] {
    println!("{str}")
  });
  clos.invoke_consume(());
}