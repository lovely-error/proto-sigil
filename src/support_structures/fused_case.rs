
// #[macro_export]
// macro_rules! case {
//   ($expr:expr =>
//     $($($pat:pat_param)|+
//       $($(if $($bool_cond:expr),+)? => $eval:expr),+);*) => {

//     match $expr {
//       router! {
//         $($($pat:pat_param)|+
//         $($(if $($bool_cond:expr),+)? => $eval:expr),+);*
//       }
//     }
//   };
// }

// macro_rules! router {
//   ($($pat:pat_param)|+
//     $(if $($bool_cond:expr),+ => $expr:expr),+ , $(tail:tt)*) => {

//     build_multi_guard_arms! {
//       $($pat)|+
//       $(if $($bool_cond),+ => $expr),+ ;
//       $($tail)*
//     }
//   };
// }

// macro_rules! build_multi_guard_arms {
//   ($($pat:pat_param)|+ ) => {};

//   ($($pat:pat_param)|+ if $($guard:expr),+ => $expr:expr , $(tail:tt)*) => {

//     $($pat)|+ if $($guard),+ => $expr ,
//     build_multi_guard_arms! { $($pat)|+ $(tail)* }
//   };
// }

// enum Test {
//   A(u8), B(u8)
// }

// fn k () {
//   let x = Test::A(1);
//   case! { x =>
//     Test::A(v) | Test::B(v)
//     if true, true => {
//       println!("{}", v);
//     },
//     if false => panic!();
//     _ => unreachable!()
//   }
// }