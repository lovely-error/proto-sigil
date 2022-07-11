
use std::{
  intrinsics::{transmute,},
  sync::atomic::{AtomicU64, Ordering, fence,},
  ptr::{null_mut,},
  alloc::{Layout, alloc, dealloc}, mem::{size_of}};

use super::action_chain::{TaskHandle, TaskMetadata};


#[repr(align(8))] #[derive(Debug, Clone, Copy)]
pub struct PageHeaderData(u64);
impl PageHeaderData {
  pub fn init_new() -> Self {
    Self(0)
  }
  pub fn is_detached(&self) -> bool {
    (self.0 & 1) == 1
  }
  pub fn get_occupation_map(&self) -> u64 {
    self.0 | 1
  }
}

const ORPHAN_PAGE : PageHeaderData = PageHeaderData(1);


pub struct GranularSlabAllocator {
  pub free_chained_pages: *mut (),
  pub b64_page_ptr: *mut (),
  pub b128_page_ptr: *mut (),
  pub b256_page_prt: *mut (),
  pub b512_page_ptr: *mut (),
}

#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum SlabSize {
  Bytes128, Bytes256, Bytes512, Bytes64
}
impl GranularSlabAllocator {
  pub fn init_new() -> Self {
    Self { free_chained_pages: null_mut(), b128_page_ptr: null_mut(),
           b256_page_prt: null_mut(), b512_page_ptr: null_mut(),
           b64_page_ptr: null_mut() }
  }
  pub fn acquire_memory(
    &mut self,
    slab_size: SlabSize
  ) -> MemorySlabControlItem {
    let page_ptr: &mut *mut ();
    let control_item: MemorySlabControlItem;
    let page_maximum: u32;
    match slab_size {
      SlabSize::Bytes128 => {
        page_ptr = &mut self.b128_page_ptr;
        page_maximum = 32;
      },
      SlabSize::Bytes256 => {
        page_ptr = &mut self.b256_page_prt ;
        page_maximum = 16;
      },
      SlabSize::Bytes512 => {
        page_ptr = &mut self.b512_page_ptr;
        page_maximum = 8;
      },
      SlabSize::Bytes64 => {
        page_ptr = &mut self.b64_page_ptr;
        page_maximum = 64 ;
      },
    }
    let mut offset: u32;
    'paging : loop {
      if *page_ptr == null_mut() { unsafe { // need some memory here
        if self.free_chained_pages != null_mut() { // have spare mem; utilise it!
          let page_after_this =
            *self.free_chained_pages.cast::<*mut ()>();
          *page_ptr = self.free_chained_pages;
          self.free_chained_pages = page_after_this;
        } else { // no spare mem; alloc!
          let fresh_page =
            alloc(Layout::from_size_align_unchecked(4096, 1));
          *page_ptr = fresh_page.cast();
        }
        // set up page header and serve a slice right away!
        *(*page_ptr).cast::<PageHeaderData>() = PageHeaderData(0b10);
        control_item = MemorySlabControlItem::init(
          (*page_ptr).cast(), 1, slab_size);
        return control_item;
      } }
      // is it possible to count trailing_ones as a single
      // bitwise op ?
      fence(Ordering::SeqCst);
      let page_header_ref = unsafe {
        &mut *(*page_ptr).cast::<AtomicU64>()
      };
      let mut page_header_ =
        page_header_ref.load(Ordering::Relaxed);
      let mut free_slab_index: u64;
      let mut page_header: PageHeaderData;
      'spininng : loop {
        page_header = PageHeaderData(page_header_);
        offset = page_header.get_occupation_map().trailing_ones();
        let is_full = offset == page_maximum;
        if is_full {
          // this page is full, detach it!
          fence(Ordering::Release);
          let _ = page_header_ref.fetch_or(1, Ordering::Relaxed);
          *page_ptr = null_mut();
          continue 'paging;
        }
        free_slab_index = 1 << offset;
        // may encounter full page
        let new = page_header_ | free_slab_index;
        let outcome =
          page_header_ref.compare_exchange_weak(
            page_header_, new,
            Ordering::Relaxed, Ordering::Relaxed);
        match outcome {
          Ok(_) => break 'paging,
          Err(actual) => {
            // someone updated the header
            if (free_slab_index & actual) == 0 {
              // but if it was caused by releasing the memory
              // then there is no conflict
              let prior = page_header_ref.fetch_or(
                free_slab_index, Ordering::Relaxed);
              if (prior & free_slab_index) != 0 {
                // someone already has this mem, rerun
                page_header_ = actual;
                continue 'spininng;
              };
              break 'paging;
            };
            page_header_ = actual;
            continue 'spininng;
          },
        }
      }
    }
    fence(Ordering::Release);
    control_item =
      MemorySlabControlItem::init(
        (*page_ptr).cast(), offset as u8, slab_size);
    return control_item;
  }
  pub fn release_memory(
    &mut self, control_item: MemorySlabControlItem
  ) { unsafe {
    let index = 1 << control_item.project_index();
    let ptr = control_item.project_base_ptr();
    let header = &mut *ptr.cast::<AtomicU64>();
    let previous = header.fetch_and(
      !index, Ordering::Relaxed);
    let xored = previous & !index;
    if xored == transmute(ORPHAN_PAGE) { // hell, yeah! free page
      *ptr.cast::<*mut ()>() = self.free_chained_pages;
      self.free_chained_pages = ptr;
    }
  } }
}

impl Drop for GranularSlabAllocator {
  fn drop(&mut self) { unsafe {
    //let mut total_of_released_pages = 0u64;
    let page_layout =
      Layout::from_size_align_unchecked(4096, 1);
    let mut page_ptr = self.free_chained_pages;
    let null_mut = null_mut();
    loop {
      if page_ptr == null_mut { break; }
      let next = *page_ptr.cast::<*mut ()>();
      dealloc(page_ptr.cast(), page_layout);
      page_ptr = next;
      //total_of_released_pages += 1;
    }
    if self.b128_page_ptr != null_mut {
      dealloc(self.b128_page_ptr.cast(), page_layout);
      //total_of_released_pages += 1;
    }
    if self.b256_page_prt != null_mut {
      dealloc(self.b256_page_prt.cast(), page_layout);
      //total_of_released_pages += 1;
    }
    if self.b512_page_ptr != null_mut {
      dealloc(self.b512_page_ptr.cast(), page_layout);
      //total_of_released_pages += 1;
    }
    //println!("Deallocated {} pages", total_of_released_pages);
  } }
}

#[derive(Debug, Clone, Copy)]
pub struct MemorySlabControlItem(u64);
impl MemorySlabControlItem {
  pub fn init_null() -> Self { Self(0) }
  pub fn is_null(&self) -> bool { self.0 == 0 }
  // maximal index is 2^6
  // maximal slab size is 2^2
  pub fn init(
    slab_ptr: *mut (), index: u8, slab_size: SlabSize
  ) -> Self { unsafe {
    let sized = ((slab_ptr as u64) << 2) +
      ((transmute::<_, u8>(slab_size) & ((1 << 2) - 1) as u8) as u64 );
    let indexed =
      (sized << 6) + (index & ((1 << 6) - 1) as u8) as u64;
    return Self(indexed)
  } }
  pub fn inject_parent_frame_ptr(&self, parent_ptr: MemorySlabControlItem) {
    let mtd_ref = self.project_metadata_mref();
    mtd_ref.parrent_frame_mtd = parent_ptr
  }
  pub fn project_metadata_mref(&self) -> &mut TaskMetadata {
    let size = self.project_size();
    let space_for_task_metadata = size_of::<TaskMetadata>();
    let offset = match size {
      SlabSize::Bytes128 => 128 ,
      SlabSize::Bytes256 => 256 ,
      SlabSize::Bytes512 => 512 ,
      SlabSize::Bytes64 => 64 ,
    } - space_for_task_metadata;
    unsafe {
      let ptr = self
        .project_slab_ptr()
        .cast::<u8>()
        .add(offset)
        .cast::<TaskMetadata>();
      return &mut *ptr;
    };
  }
  pub fn project_size(&self) -> SlabSize {
    unsafe { transmute(((self.0 >> 6) as u8) & (((1 << 2) - 1)) as u8) }
  }
  pub fn project_index(&self) -> u8 {
    (self.0 as u8) & (((1 << 6) - 1) as u8)
  }
  pub fn project_base_ptr(&self) -> *mut () {
    (self.0 >> 8) as *mut _
  }
  pub fn project_slab_ptr(&self) -> *mut () {
    let size = match self.project_size() {
      SlabSize::Bytes64 => 1,
      SlabSize::Bytes128 => 2,
      SlabSize::Bytes256 => 4,
      SlabSize::Bytes512 => 8,
    };
    let index = self.project_index() as usize;
    let base_ptr = self.project_base_ptr();
    return unsafe {
      base_ptr
      .cast::<[u64;8]>()
      .add(index * size)
      .cast::<()>()
    }
  }
}


pub struct RCTaskBox<T> {
  pub(super) storage_ptr: *mut (T, MemorySlabControlItem, u64)
}

impl <T> RCTaskBox<T> {
  pub fn init(handle: TaskHandle, value: T) -> Self {
    unsafe {
      let size = match size_of::<(T, MemorySlabControlItem, u64)>() {
        0 ..= 64 => SlabSize::Bytes64,
        0 ..= 128 => SlabSize::Bytes128,
        0 ..= 256 => SlabSize::Bytes256,
        0 ..= 512 => SlabSize::Bytes512,
        _ => panic!("Value is too big to fit into task local box")
      };
      let mem = handle.request_slab(size);
      let ptr =
        mem.project_slab_ptr().cast::<(T, MemorySlabControlItem, u64)>();
      ptr.write((value, mem, 0));
      return RCTaskBox { storage_ptr: ptr }
    }
  }
  pub fn get_ref(&self) -> &T { unsafe {
    &*self.storage_ptr.cast::<u64>().add(2).cast::<T>()
  } }
  pub fn try_unbox(self, handle: TaskHandle) -> Option<T> { unsafe {
    let (_, mci, rc) = &*self.storage_ptr;
    if *rc == 0 {
      let val = self.storage_ptr.cast::<T>().read();
      (&mut *handle.2).release_memory(*mci);
      return Some(val);
    } else {
      return None
    }

  } }
  pub fn vend_ref(&self) -> RcTaskBoxRef<T> {
    unsafe {
      let (_, _, counter) = &mut *self.storage_ptr;
      *counter += 1;
    };
    return RcTaskBoxRef { storage_ptr: self.storage_ptr }
  }
}

impl <T> Drop for RCTaskBox<T> {
  fn drop(&mut self) { unsafe {
    let rc = *self.storage_ptr.cast::<u64>();
    if rc == 0 {
      let (_, mci, val) = self.storage_ptr.read();
      drop(val);
      let mask = 1u64 << mci.project_index();
      let header = &*mci.project_base_ptr().cast::<AtomicU64>();
      let _ = header.fetch_xor(mask, Ordering::Relaxed);
      // page may leak, but it is very unlikely
    }
  } }
}

#[derive(Debug)]
pub struct RcTaskBoxRef<T> {
  pub(super) storage_ptr: *mut (T, MemorySlabControlItem, u64)
}
impl <T> RcTaskBoxRef<T> {
  pub fn get_ref(&self) -> &T { unsafe {
    &*self.storage_ptr.cast::<u64>().add(2).cast::<T>()
  } }
}
impl <T> Clone for RcTaskBoxRef<T> {
  fn clone(&self) -> Self {
    unsafe { *self.storage_ptr.cast::<u64>() += 1 };
    return Self { storage_ptr: self.storage_ptr }
  }
}

impl <T> Drop for RcTaskBoxRef<T> {
  fn drop(&mut self) { unsafe {
    *self.storage_ptr.cast::<u64>() -= 1;
    let rc = *self.storage_ptr.cast::<u64>();
    if rc == 0 {
      let (_, mci, val) = self.storage_ptr.read();
      drop(val);
      let mask = 1u64 << mci.project_index();
      let header = &*mci.project_base_ptr().cast::<AtomicU64>();
      let _ = header.fetch_xor(mask, Ordering::Relaxed);
      // page may leak, but it is very unlikely
    }
  } }
}