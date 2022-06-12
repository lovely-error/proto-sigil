
#[macro_export]
macro_rules! task {
  ($($tokens:tt)*) => {
    (||{
      task_unfolding_loop! { $($tokens)* }
    })()
  };
}
#[macro_export]
macro_rules! task_unfolding_loop {
  (async let $binder:pat = $expr:expr ; $($tail:tt)*) => {

    task_unfolding_loop! { $($tail)* }
  };
  ($(go $expr:expr ;)* $($tail:tt)*) => {
    // da heck?!!
    //task_unfolding_loop! { $($tail)* }
  };
  ($($stmt:stmt ;)* $($tail:tt)*) => {

    task_unfolding_loop! { $($tail)* }
  };
  () => {};
}

fn test () { task! {
  async let () = () ;
  let a = 0 ;
  go println!("{}", a) ;
} }