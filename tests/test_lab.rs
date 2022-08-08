

use std::{
  alloc::dealloc, mem::{size_of, align_of, ManuallyDrop, MaybeUninit,},
  ptr::{null_mut, addr_of_mut}, hash::{Hash, Hasher}, collections::hash_map::DefaultHasher, path::PathBuf};

use proto_sigil::{elaborator::{
  worker::{WorkGroup, WorkGroupRef}, self,}, support_structures::universal_bitwise_conversion::bitcast,
};



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

// #[test]
fn par () {
  let path = PathBuf::from("/Users/cromobeingnur/testim_sigi");
  let wg = elaborator::main::elab_invocation_setup(path);
  let executor = WorkGroupRef::init(6, wg);
  executor.await_completion();
}

#[test]
fn test () {
  #[repr(packed)]
  struct  H {
    b: u16,
    a: bool,
  }
  println!("Size of H is {}", size_of::<H>())
}

fn nullify<T>(val:T) -> T {
  let size_of = size_of::<T>();
  let mut val = val;
  let ptr = addr_of_mut!(val).cast::<u8>() ;
  for i in 0 .. size_of {
    unsafe { *ptr.add(i) = 0 };
  }
  return val;
}

#[derive(Debug, PartialEq)]
struct Test {
  a: u16,
  b: bool,
}

#[test]
fn byte_reading_works () {
  let nullified = nullify(Test {a:u16::MAX, b:true});
  // println!("{:#?}", nullified)
  assert!(nullified == Test {a:0,b:false})
}


#[test]
fn bitcasting () {
  fn ll<T>(val:T) {
    let val_bits = unsafe { bitcast::<_, [u8;3]>(val) };
    // println!("{:#?}", val_bits)
    assert!([1,255,255] == val_bits)
  }
  #[repr(packed)]
  struct J { a: bool, b: u16 }

  ll(J {a:true, b:u16::MAX});
}

// #[test]
// fn what () {
//   #[repr(C)]
//   union K {
//     word: u64,
//     byte: u8,
//   }
//   let mut val = MaybeUninit::zeroed();
//   unsafe {
//     *val.as_mut_ptr() = K { word: u64::MAX };
//     *val.as_mut_ptr() = K { byte: 0x0F };
//   }
//   let val = unsafe { val.assume_init().word };
//   println!("{}", val);
//   assert!(val == !0xFF + 0x0F);
// }