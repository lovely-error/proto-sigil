
use std::{
  marker::PhantomData,
  hash::{Hash, Hasher}, collections::hash_map::DefaultHasher,
  alloc::{Layout, alloc, dealloc}, mem::{size_of, align_of, needs_drop,},
  sync::atomic::{AtomicU64, Ordering, AtomicBool, fence},
  ptr::drop_in_place, };



// Associative table that is concurrently accessible for writes and reads.
// All inserted items reside at stable addresses.
pub struct PasteboardTable<Key: Hash, Value> {
  head_ptr: *mut (),
  least_crowded_page_ptr: AtomicU64,
  is_frozen: AtomicBool,
  _own_values_invariantly: PhantomData<Value>,
  _own_keys_invariantly: PhantomData<Key>,
}

#[derive(Debug)]
struct BucketHeader {
  next_bucket_ptr: AtomicU64,
  occupation_map: AtomicU64,
}
impl BucketHeader {
  pub fn init_new() -> Self {
    Self { next_bucket_ptr:AtomicU64::new(0),
           occupation_map: AtomicU64::new(0) }
  }
  pub fn bucket_layout_for<V>() -> Layout { unsafe {
    let layout =
      Layout::from_size_align_unchecked(
        size_of::<(u64, V)>() * 64, align_of::<(u64, V)>());
    return layout;
  } }
}


impl <K: Hash, V> PasteboardTable<K, V> {
  fn allocate_bucket() -> *mut () { unsafe {
    let layout =
      BucketHeader::bucket_layout_for::<V>();
    let page = alloc(layout);
    *page.cast::<BucketHeader>() = BucketHeader::init_new();
    return page.cast::<()>()
  } }
  fn handle_storage_shortage(bucket_ptr: *mut ()) { unsafe {
    let new_page = Self::allocate_bucket() as u64;
    let mut bucket_ptr = bucket_ptr;
    'here : loop {
      let header = &*bucket_ptr.cast::<BucketHeader>();
      let outcome =
      (&header.next_bucket_ptr).compare_exchange_weak(
        0, new_page,
        Ordering::Relaxed, Ordering::Relaxed);
      match outcome {
        Ok(_) => break 'here,
        Err(new) => {
          bucket_ptr = new as *mut ()
        },
      }
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
  // pub fn is_empty(&self) -> bool { unsafe {
  //   let number =
  //     (&*self.head_ptr.cast::<AtomicU64>()).load(Ordering::Relaxed);
  //   let occupation_map =
  //     transmute::<_, BucketHeader>(number);
  //   return occupation_map.bank_is_empty();
  // } }
  fn compress(&self) { unsafe {

    fn pull_in<V>(
      index: u64, initial_offset: u64, dest: *mut (), source: *mut ()
    ) { unsafe {
      let offset = 1 << index;
      let occupation_map_here =
        (&*dest.cast::<BucketHeader>())
        .occupation_map.load(Ordering::Relaxed);
      let mut source = source;
      loop {
        if source.is_null() { return; }
        let header =
          (&*source.cast::<BucketHeader>())
          .occupation_map.load(Ordering::Relaxed);
        let something_here = (header & offset) != 0;
        if something_here {
          let val =
            source.cast::<(u64, V)>()
            .add((index + initial_offset) as usize).read();
          dest.cast::<(u64, V)>()
          .add((index + initial_offset) as usize).write(val);

          let new_occup_here = occupation_map_here | offset;
            (&*dest.cast::<BucketHeader>())
            .occupation_map.store(
              new_occup_here, Ordering::Relaxed);

          let new_map_for_source = header & !offset;
          (&*source.cast::<BucketHeader>())
            .occupation_map.store(
              new_map_for_source, Ordering::Relaxed);
          return ;
        }
        let next =
          (&*source.cast::<BucketHeader>())
          .next_bucket_ptr.load(Ordering::Relaxed);
        source = next as *mut ();
      }
    } }


    if !self.is_frozen.load(Ordering::Relaxed) {
      panic!("Cant perform defragmentation on nonfrozen table!")
    }
    let number_of_free_slots =
      64 - (16 / size_of::<(u64, V)>().max(1) as u64);
    let initial_index = 64 - number_of_free_slots;

    let mut base_ptr = self.head_ptr;
    loop {
      let drain_bucket_ptr =
        (&*base_ptr.cast::<BucketHeader>()).next_bucket_ptr
        .load(Ordering::Relaxed) as *mut ();

      if drain_bucket_ptr.is_null() { return; }

      let occupation_map_here =
        (&*base_ptr.cast::<BucketHeader>())
        .occupation_map.load(Ordering::Relaxed);

      let mut index = 0;
      loop {
        let offset = 1 << index;
        let free_slot_here = (occupation_map_here & offset) == 0;
        if free_slot_here {
          pull_in::<V>(
            index, initial_index,
            base_ptr, drain_bucket_ptr);
        }
        index += 1;
        if index == number_of_free_slots {
          break;
        }
      }
      base_ptr = drain_bucket_ptr;
    }
  } }
  fn shrink(&self) { unsafe {
    let mut page_ptr = self.head_ptr;
    loop {
      let header =
        (&*page_ptr.cast::<BucketHeader>())
        .occupation_map.load(Ordering::Relaxed);
      if header == 0 { break; }
      else {
        let next =
          (&*page_ptr.cast::<BucketHeader>())
          .next_bucket_ptr.load(Ordering::Relaxed);
        if next == 0 { return }
        page_ptr = next as *mut ();
      }
    }
    loop {
      let next =
        (&*page_ptr.cast::<BucketHeader>())
        .next_bucket_ptr.load(Ordering::Relaxed);
      dealloc(
        page_ptr.cast(), BucketHeader::bucket_layout_for::<V>());
      if next == 0 { return; }
      page_ptr = next as *mut ();
    }
  } }
  pub fn freeze(&self) {
    self.is_frozen.store(true, Ordering::Relaxed);
    self.least_crowded_page_ptr.store(0, Ordering::Relaxed);
    fence(Ordering::Release);
    self.compress();
    //self.shrink();
  }
  // O(1 + k)
  // where k denote some amount of being unlucky.
  // most of the time is expected to be 0 or low
  pub fn insert(&self, key: &K, value: V) { unsafe {
    if self.is_frozen.load(Ordering::Relaxed) {
      panic!("Cant insert into frozen table!");
    }
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    let number_of_free_slots =
      64 - (16 / size_of::<(u64, V)>().max(1) as u64);
    let offset = hash % number_of_free_slots;
    let index = 1 << offset;

    let mut bucket_ptr =
      self.least_crowded_page_ptr.load(Ordering::Relaxed) as *mut ();

    let mut occupation_map: u64;
    let mut header: &BucketHeader;
    'fetching : loop {
      loop {
        fence(Ordering::SeqCst);
        header = &*bucket_ptr.cast::<BucketHeader>();
        occupation_map =
          header.occupation_map.load(Ordering::Relaxed);

        let collided = (occupation_map & index) != 0;
        if collided { // goto next bucket
          fence(Ordering::Acquire);
          let next =
            header.next_bucket_ptr.load(Ordering::Relaxed)
            as *mut ();
          if next.is_null() { // need more storage
            Self::handle_storage_shortage(bucket_ptr);
            fence(Ordering::Release);
          } else {
            bucket_ptr = next;
          }
          let page_is_overpopulated =
            occupation_map.count_ones() > 54;
          if page_is_overpopulated {
            // change write ptr to next page
            self.least_crowded_page_ptr.store(
              bucket_ptr as u64, Ordering::Relaxed);
          }
        } else { break }
      }
      fence(Ordering::SeqCst);
      let updated_occupation_map = occupation_map | index;
      let update_outcome =
      header.occupation_map.compare_exchange_weak(
        occupation_map, updated_occupation_map,
        Ordering::Relaxed, Ordering::Relaxed);
      match update_outcome {
        Ok(_) => break 'fetching,
        Err(new) => {
          if (new & index) == 0 {
            (&*header).occupation_map.fetch_or(
              index, Ordering::Relaxed);
              //fence(Ordering::Release);
            break 'fetching;
          } else {
            continue 'fetching;
          }
        },
      }
    }
    fence(Ordering::Release);
    bucket_ptr.cast::<(u64, V)>()
    .add(((64 - number_of_free_slots) + offset) as usize)
    .write((hash, value));

  } }
  // O(n / (63 - m) - k)
  // Sensitive to the how far from head item being retrieved is located.
  // The farther it is, the worse performance this operation has.
  // On average, this is not too bad for realistic amounts.
  // For 5000 items, retrieve time for randomly selected keys is 6 ms.
  //
  // If rust had 16 byte atomics the complexity could be lowered
  // to O(n / 95 - k) and retrieve time would be 50% better.
  // Somewhat like 3 ms.
  // And if simd was stable, this could be 4 times faster.
  pub fn retrieve_ref(&self, key: &K) -> Option<&V> { unsafe {

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    let number_of_free_slots =
      64 - (16 / size_of::<(u64, V)>().max(1) as u64);
    let offset = hash % number_of_free_slots;
    let index = 1 << offset;

    let mut bucket_ptr = self.head_ptr;
    loop {
      let header = &*bucket_ptr.cast::<BucketHeader>();
      let occupation_map =
        (&header.occupation_map).load(Ordering::Relaxed);
      let item_is_here = (occupation_map & index) != 0;
      if item_is_here {
        let (stored_key_hash, value) =
          &*bucket_ptr.cast::<(u64, V)>()
          .add(((64 - number_of_free_slots) + offset) as usize);
        if *stored_key_hash == hash {
          return Some(value);
        }
      } else {
        if self.is_frozen.load(Ordering::Relaxed) {
          return None;
        }
      }
      fence(Ordering::Release);
      let next_bucket_ptr =
        (&header.next_bucket_ptr).load(Ordering::Relaxed) as *mut ();
      if next_bucket_ptr.is_null() { return None; };
      bucket_ptr = next_bucket_ptr;
    }

  }; }
}

impl <K: Hash, V> Drop for PasteboardTable<K, V> {
  fn drop(&mut self) { unsafe {
    let layout =
      BucketHeader::bucket_layout_for::<V>();
    let number_of_free_slots =
      64 - (16 / size_of::<(u64, V)>().max(1) as u64);

    let mut bucket_ptr = self.head_ptr;
    'list_release : loop {
      let header = &*bucket_ptr.cast::<BucketHeader>();

      if needs_drop::<V>() {
        let occupation_map =
          (&header.occupation_map).load(Ordering::Relaxed);
        let item_slot =
          bucket_ptr.cast::<(u64, V)>().add(1);
        let mut slot_index = 0;
        'bucket_value_dropping : loop {
          let offset = 1 << slot_index;
          let something_is_here = (occupation_map & offset) != 0;
          if something_is_here {
            let value_ptr =
              item_slot.cast::<u64>().add(1).cast::<V>();
            drop_in_place(value_ptr);
          }
          slot_index += 1;
          if slot_index == number_of_free_slots {
            break 'bucket_value_dropping;
          }
        }
      }
      fence(Ordering::Release);
      dealloc(bucket_ptr.cast(), layout);

      let next_bucket =
        (&header.next_bucket_ptr).load(Ordering::Relaxed)
        as *mut ();
      if next_bucket.is_null() { break 'list_release; }
      bucket_ptr = next_bucket;
    }
  } }
}


unsafe impl <K: Hash, V> Send for PasteboardTable<K, V> {}
unsafe impl <K: Hash, V> Sync for PasteboardTable<K, V> {}