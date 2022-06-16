
use std::{
  marker::PhantomData,
  hash::{Hash, Hasher}, collections::hash_map::DefaultHasher,
  alloc::{Layout, alloc}, mem::{size_of, align_of},
  sync::atomic::{AtomicU64, Ordering, AtomicBool},
  intrinsics::transmute, };

use crate::parser::node_allocator::EntangledPtr;


// Concurrently accessible for writes and reads, but
// only if they dont overlap.
// Inteded use is to proceed in two stages.
// In first stage, multiple threads populate this
// object, then freeze it. Reads do not occure at this stage.
// After freezing, mutation of this object is forbiden.
// In second stage multiple threads may read it without fear of
// of getting corrupted data.
pub struct PersistentTable<Key: Hash, Value> {
  head_ptr: *mut (),
  least_crowded_page_ptr: AtomicU64,
  is_frozen: AtomicBool,
  _own_values_invariantly: PhantomData<Value>,
  _own_keys_invariantly: PhantomData<Key>,
}

#[derive(Debug, Clone, Copy)]
struct BucketHeader(u64);
impl BucketHeader {
  pub fn init_new() -> Self {
    Self(0)
  }
  pub fn project_occupation_map(&self) -> u32 {
    self.0 as u32
  }
  pub fn bank_is_empty(&self) -> bool {
    self.0 == 0
  }
  pub fn project_next_page_eptr(&self) -> EntangledPtr {
    unsafe { transmute((self.0 >> 32) as u32) }
  }
  pub fn inject_occupation_map(&mut self, map: u32) {
    self.0 = self.0 | (map as u64)
  }
}


impl <K: Hash, V> PersistentTable<K, V> {
  fn allocate_bucket() -> *mut () { unsafe {
    let layout =
      Layout::from_size_align_unchecked(
        size_of::<(u64, V)>() * 32, align_of::<(u64, V)>());
    let page = alloc(layout);
    *page.cast::<BucketHeader>() = BucketHeader::init_new();
    return page.cast::<()>()
  } }
  fn handle_storage_shortage(bucket_ptr: *mut ()) { unsafe {
    let new_page = Self::allocate_bucket();
    let mut bucket_ptr = bucket_ptr;
    loop {
      let header_ref = &*bucket_ptr.cast::<AtomicU64>();
      let header = header_ref.load(Ordering::Relaxed);
      let header = transmute::<_, BucketHeader>(header);
      let next_bucket_ptr = header.project_next_page_eptr();
      let no_page_here = next_bucket_ptr.is_null();
      if no_page_here {
        let entp =
          EntangledPtr::from_ptr_pair(
            bucket_ptr, new_page).unwrap();
        let entp = (transmute::<_, u32>(entp) as u64) << 32;

        let _ = header_ref.fetch_or(entp, Ordering::Relaxed); // valid?
        return;
      }
      let next_bucket =
        next_bucket_ptr.reach_referent_from(bucket_ptr);
      bucket_ptr = next_bucket;
    }
  } }
  pub fn init() -> Self { unsafe {
    let page = Self::allocate_bucket();
    *page.cast::<BucketHeader>() = BucketHeader::init_new();
    return Self { head_ptr: page.cast(),
                  _own_values_invariantly: PhantomData,
                  least_crowded_page_ptr: AtomicU64::new(page as u64),
                  _own_keys_invariantly: PhantomData,
                  is_frozen: AtomicBool::new(false) }
  } }
  pub fn is_empty(&self) -> bool { unsafe {
    let number =
      (&*self.head_ptr.cast::<AtomicU64>()).load(Ordering::Relaxed);
    let occupation_map =
      transmute::<_, BucketHeader>(number);
    return occupation_map.bank_is_empty();
  } }
  pub fn freeze(&self) {
    self.is_frozen.store(true, Ordering::Relaxed)
  }
  pub fn insert(&self, key: &K, value: V) { unsafe {
    if self.is_frozen.load(Ordering::Relaxed) {
      panic!("Cant insert into frozen table!");
    }
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    let offset = hash % 31; // this is not 32 because
                                 // space is needed for page header
    let index = 0b10u32 << offset;

    let mut bucket_ptr =
      self.least_crowded_page_ptr.load(Ordering::Relaxed) as *mut ();

    let mut occupation_map: u32;
    let mut header: &AtomicU64;
    loop {
      header = &*bucket_ptr.cast::<AtomicU64>();
      let header_ =
        transmute::<_, BucketHeader>(header.load(Ordering::Relaxed));
      occupation_map =
        header_.project_occupation_map();

      let collided = (occupation_map & index) != 0;
      if collided { // goto next bucket
        let eptr = header_.project_next_page_eptr();
        if eptr.is_null() { // need more storage
          Self::handle_storage_shortage(bucket_ptr)
        }
        let next_page = eptr.reach_referent_from(bucket_ptr);
        bucket_ptr = next_page;
        let page_is_overpopulated =
          occupation_map.count_ones() > 28;
        if page_is_overpopulated {
          // change write ptr to next page
          self.least_crowded_page_ptr.store(
            next_page as u64, Ordering::Relaxed);
        }
      } else { break }
    }

    let updated_occupation_map = (occupation_map | index) as u64;
    let _ = header.fetch_or(
      updated_occupation_map, Ordering::Relaxed);

    bucket_ptr.cast::<(u64, V)>()
    .add(1 + offset as usize).write((hash, value));

  } }

  pub fn retrieve_ref(&self, key: &K) -> Option<&V> { unsafe {
    if !self.is_frozen.load(Ordering::Relaxed) {
      panic!("Table cannot be read in unfrozen state")
    };
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    let offset = hash % 31; // this is not 32 because
                                 // space is needed for page header
    let index = 0b10u32 << offset;

    let mut bucket_ptr = self.head_ptr;
    loop {
      let header = &*bucket_ptr.cast::<AtomicU64>();
      let header =
        transmute::<_, BucketHeader>(header.load(Ordering::Relaxed));
      let occupation_map = header.project_occupation_map();
      let item_is_here = (occupation_map & index) != 0;
      if item_is_here {
        let (stored_key_hash, value) =
          &*bucket_ptr.cast::<(u64, V)>().add(1 + offset as usize);
        if *stored_key_hash == hash {
          return Some(value);
        }
      }
      let next_bucket_ptr = header.project_next_page_eptr();
      if next_bucket_ptr.is_null() { return None; };
      bucket_ptr = next_bucket_ptr.reach_referent_from(bucket_ptr);
    }

  }; }
}


unsafe impl <K: Hash, V> Send for PersistentTable<K, V> {}
unsafe impl <K: Hash, V> Sync for PersistentTable<K, V> {}