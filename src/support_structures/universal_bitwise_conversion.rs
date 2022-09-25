use std::mem::{ManuallyDrop, MaybeUninit};

#[repr(C)]
union CastHack<S, D> {
  src: ManuallyDrop<S>,
  dst: ManuallyDrop<D>,
}

// this can convert things that transmute cannot handle
pub unsafe fn bitcast<S, D>(val:S) -> D {
  let zeroed =
    MaybeUninit::<CastHack<S, D>>::zeroed();
  let mut that = zeroed.assume_init();
  that.src = ManuallyDrop::new(val);
  return ManuallyDrop::into_inner(that . dst)
}