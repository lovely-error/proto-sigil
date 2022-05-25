
#[macro_export]
macro_rules! packed_struct {
  ($name:ident $(: $lit:literal bytes)? => $($tail:tt)+) => {
    struct $name ();
    impl $name {
      inner_unfolding_loop! { $($tail)+ }
    }
  };
}
macro_rules! inner_unfolding_loop {
  ($field_id:ident : $type:ty | $size:literal bits $($tail:tt)*) => {

    inner_unfolding_loop! { $($tail)* }
  };
  () => {};
}
macro_rules! unfold_constituents {
  (0) => {};
  ($expr:expr) => {

  };
}

packed_struct! {
  Example : 8 bytes =>
    v1 : *mut u8 | 40 bits
    v2 : u32 | 24 bits
}