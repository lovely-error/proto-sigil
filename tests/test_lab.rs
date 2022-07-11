

use std::{
  alloc::dealloc, mem::{size_of, align_of,},
  ptr::{null_mut}, hash::{Hash, Hasher}, collections::hash_map::DefaultHasher};

use proto_sigil::{elaborator::{
  worker::{WorkGroup},},};


  
static mut FLAG : bool = false;

struct Example(bool);
impl Drop for Example {
  fn drop(&mut self) {
    unsafe { FLAG = true };
  }
}

#[test]
fn drop_on_ptrs () {
  use std::alloc::{alloc, Layout};

  unsafe {
    let lay = Layout::new::<Example>();
    let mem_ptr = alloc(lay);
    *mem_ptr.cast::<Example>() = Example(true);
    // this does mean that writing through ptr deref may drop garbage.
    assert_eq!(FLAG, true);
    //mem_ptr.cast::<Example>().write(Example(true));
    dealloc(mem_ptr, lay);
  };
}


//#[test]
fn size_test () {
  println!("{}", size_of::<Box<WorkGroup>>())
}



//#[test]
fn byte_order () {
  println!("{:#066b}" , 1);
  println!("{:#010b}", 0u8 ^ 1 << 2);
  println!( "{}", (!(1u8 << 2)) .trailing_ones() );
}

fn scope () {
  {
    fn func1() {}
  };
  {
    fn func1() {}
  };
}

//#[test]
fn hhh () {
  println!("{}", align_of::<[u8;3]>())
}



//#[test]
fn p () {
  let str = "aoao".to_string();
  let str2 = "oaoa".to_string();
  let mut hasher = DefaultHasher::new();
  str.hash(&mut hasher);
  let hash1 = hasher.finish();
  println!("{}", hash1);
  let mut hasher = DefaultHasher::new();
  str2.hash(&mut hasher);
  let hash2 = hasher.finish();
  println!("{}", hash2);

  println!("Rem {}", hash1 % 32);
  println!("Rem {}", hash2 % 32);

}

//#[test]
fn simd () {
  // use std::simd;

}



#[test]
fn read_zst_from_null () {
  let inv : *mut () = null_mut();
  let () = unsafe { inv.read() };
}

#[test]
fn count_ones () {
  let num = !0u64;
  let count = num.trailing_ones();
  println!("{count}")
}