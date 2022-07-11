
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
  (fix let $binder:pat = $expr:expr ; $($tail:tt)*) => {

    task_unfolding_loop! { $($tail)* }
  };
  (await spawned ; $($tail:tt)*) => {

    task_unfolding_loop! { $($tail)* }
  };
  ($stmt:stmt ; $($tail:tt)*) => {

    task_unfolding_loop! { $($tail)* }
  };
  () => {};
}

fn test () { task! {
  fix let () = () ;
  let a = 0 ;
  loop { break } ;
  if false { () } else { () } ;
  await spawned ;
} }