use std::mem::ManuallyDrop;

#[repr(C)]
union CastHack<S, D> {
  src: ManuallyDrop<S>,
  dst: ManuallyDrop<D>,
}
pub unsafe fn bitcast<S, D>(val:S) -> D {
  ManuallyDrop::into_inner(
    CastHack { src: ManuallyDrop::new(val) } . dst)
}