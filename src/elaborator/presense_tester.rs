
use std::{
  marker::PhantomData, hash::{Hash, Hasher},
  sync::atomic::{AtomicU64, AtomicPtr, Ordering, fence, },
  alloc::{Layout, alloc},
  ptr::null_mut, collections::hash_map::DefaultHasher,};


struct Header {
  next: AtomicPtr<Header>,
  occopation_map: AtomicU64,
}

pub struct PresenseSet<T: Hash> {
  storage_ptr: *mut Header,
  _mark: PhantomData<T>,
}
impl <T: Hash> PresenseSet<T> {
  fn alloc_bucket() -> *mut () { unsafe {
    let page =
      alloc(Layout::from_size_align_unchecked(512, 8));
    *page.cast::<Header>() =
      Header {
        next: AtomicPtr::new(null_mut()), occopation_map: AtomicU64::new(0) };
    return page.cast();
  } }
  pub fn init() -> Self {
    let page = Self::alloc_bucket();
    return Self { storage_ptr: page.cast(), _mark: PhantomData };
  }
  fn handle_mem_shortage(start: *mut Header) { unsafe {
    let bucket = Self::alloc_bucket();
    let mut ptr = start;
    'here : loop {
      let outcome = (&*ptr).next.compare_exchange(
        null_mut(), bucket.cast(),
        Ordering::Relaxed, Ordering::Relaxed);
      match outcome {
        Ok(_) => {
          break 'here;
        },
        Err(recent) => {
          ptr = recent
        },
      }
    }
  } }
  // this proc return true if smth was already checked in
  pub fn check_in(&self, item: &T) -> bool { unsafe {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);
    let hash = hasher.finish();
    let index = hash % 61;
    let mask = 1u64 << index;

    let mut bucket_ptr = self.storage_ptr;
    loop {
      let occup_map = &(&*bucket_ptr).occopation_map;
      let outcome =
        occup_map.fetch_or(mask, Ordering::Relaxed);
      let hash_ptr =
        bucket_ptr.cast::<u64>().add(2 + index as usize);
      if (outcome & mask) != 0 { // smth was there
        if *hash_ptr == hash { return true; }
        else { // look in next bucket
          loop {
            let next =
              (&*bucket_ptr).next.load(Ordering::Relaxed);
            if next.is_null() {
              Self::handle_mem_shortage(bucket_ptr);
              continue;
            }
            bucket_ptr = next;
            break;
          }
        }
      } else { // free spot
        *hash_ptr = hash;
        fence(Ordering::Release);
        return false;
      }
    }
  } }
  pub fn check_out(&self, item: &T) -> bool { unsafe {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);
    let hash = hasher.finish();
    let index = hash % 61;
    let mask = 1u64 << index;

    let mut bucket_ptr = self.storage_ptr;
    loop {
      let occup_map =
        (&*bucket_ptr).occopation_map.load(Ordering::Relaxed);
      if (occup_map & mask) != 0 {
        fence(Ordering::Acquire);
        let hash_there =
          *bucket_ptr.cast::<u64>().add(2 + index as usize);
        if hash_there == hash {
          return true;
        } else {
          let next =
            (&*bucket_ptr).next.load(Ordering::Relaxed);
          if next.is_null() {
            return false;
          } else {
            bucket_ptr = next;
          }
        }
      } else {
        return false
      }
    }
  } }
}


unsafe impl <T: Hash> Sync for PresenseSet<T> {}