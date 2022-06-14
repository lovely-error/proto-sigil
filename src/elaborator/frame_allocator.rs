
use std::{
  intrinsics::transmute,
  sync::atomic::{AtomicU64, Ordering, fence},
  ptr::null_mut,
  alloc::{Layout, alloc, dealloc}};


#[repr(align(8))]
pub struct PageHeaderData {
  pub occupation_map: u32,
  pub is_detached: bool,
  pub _padding: [u8;3]
}
impl PageHeaderData {
  pub fn init_new() -> Self {
    Self { occupation_map: 1, is_detached: false, _padding: [0;3] }
  }
}

const ORPHAN_PAGE : PageHeaderData =
  PageHeaderData {_padding: [0;3], is_detached: true, occupation_map: 1};


pub struct GranularSlabAllocator {
  pub free_page_list: *mut (),
  pub b128_page_ptr: *mut (),
  pub b256_page_prt: *mut (),
  pub b512_page_ptr: *mut (),
}

#[repr(u8)] #[derive(Debug, Clone, Copy)]
pub enum SlabSize {
  Bytes128, Bytes256, Bytes512
}
impl GranularSlabAllocator {
  pub fn init_new() -> Self {
    Self { free_page_list: null_mut(), b128_page_ptr: null_mut(),
           b256_page_prt: null_mut(), b512_page_ptr: null_mut() }
  }
  pub fn acquire_memory(
    &mut self,
    slab_size: SlabSize
  ) -> MemorySlabControlItem {
    let page_ptr: &mut *mut ();
    let mult: usize;
    let control_item: MemorySlabControlItem;
    match slab_size {
      SlabSize::Bytes128 => {
        page_ptr = &mut self.b128_page_ptr;
        mult = 2;
      },
      SlabSize::Bytes256 => {
        page_ptr = &mut self.b256_page_prt ;
        mult = 4;
      },
      SlabSize::Bytes512 => {
        page_ptr = &mut self.b512_page_ptr;
        mult = 8;
      },
    }
    if *page_ptr == null_mut() { unsafe { // need some memory here
      if self.free_page_list != null_mut() { // have spare mem; utilise it!
        let page_after_this = *self.free_page_list.cast::<*mut ()>();
        *page_ptr = self.free_page_list;
        self.free_page_list = page_after_this;
      } else { // no spare mem; alloc!
        let fresh_page =
          alloc(Layout::from_size_align_unchecked(4096, 1));
        *page_ptr = fresh_page.cast();
      }
      // set up page header and serve a slice right away!
      *(*page_ptr).cast::<PageHeaderData>() = PageHeaderData {
        _padding: [0;3], is_detached: false, occupation_map: 0b11 };
      let slab_ptr = (*page_ptr).cast::<u64>().add(2 * mult);
      control_item = MemorySlabControlItem::init(
        slab_ptr.cast(), 1, slab_size);
      return control_item;
    } }
    // is it possible to count trailing_ones as a single
    // bitwise op ?
    fence(Ordering::AcqRel);
    let page_header_ref = unsafe {
      &mut *(*page_ptr).cast::<AtomicU64>() };
    let mut page_header_ =
      page_header_ref.load(Ordering::Acquire);
    let mut offset: u32;
    let mut free_slab_index: u64;
    let mut page_header: PageHeaderData;
    'spininng : loop {
      page_header = unsafe {
        transmute::<_, PageHeaderData>(page_header_) };
      offset = page_header.occupation_map.trailing_ones();
      free_slab_index = 1 << offset;
      let new = page_header_ | free_slab_index;
      let outcome =
        page_header_ref.compare_exchange_weak(
          page_header_, new,
          Ordering::Release, Ordering::Relaxed);
      match outcome {
        Ok(_) => break 'spininng,
        Err(actual) => {
          // someone updated the header
          if free_slab_index & actual == 0 {
            // but if it was caused by sombody releasing the memory
            // then there is no conflict
            let _ = page_header_ref.fetch_xor(
              free_slab_index, Ordering::Relaxed); // Release ???
            break 'spininng;
          };
          page_header_ = actual;
        },
      }
    }
    if page_header.occupation_map == u32::MAX {
      // whoa! this page is full, detach it!
      let _ = page_header_ref.fetch_xor(
        1 << 32, Ordering::Relaxed);
      *page_ptr = null_mut();
    }
    let slab_ptr = unsafe {
      (*page_ptr).cast::<u64>()
      .add(mult * offset as usize).cast::<()>()
    };
    control_item =
      MemorySlabControlItem::init(
        slab_ptr, free_slab_index as u8, slab_size);
    return control_item;
  }
  pub fn release_memory(
    &mut self, control_item: MemorySlabControlItem
  ) { unsafe {
    let index = 1 << control_item.project_index();
    let ptr = control_item.project_ptr();
    let header = &mut *ptr.cast::<AtomicU64>();
    let previous = header.fetch_xor(index, Ordering::Relaxed);
    if previous ^ index == transmute(ORPHAN_PAGE) { // hell, yeah! free page
      *ptr.cast::<*mut ()>() = self.free_page_list;
      self.free_page_list = ptr;
    }
  } }
}

impl Drop for GranularSlabAllocator {
  fn drop(&mut self) { unsafe {
    //let mut total_of_released_pages = 0u64;
    let page_layout =
      Layout::from_size_align_unchecked(4096, 1);
    let mut page_ptr = self.free_page_list;
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
      ((transmute::<_, u8>(slab_size) & ((1 << 2) - 1)) as u64 );
    let indexed =
      (sized << 6) + (index & ((1 << 6) - 1)) as u64;
    return Self(indexed)
  } }
  pub fn project_size(&self) -> SlabSize {
    unsafe { transmute(((self.0 >> 6) as u8) & ((1 << 2) - 1)) }
  }
  pub fn project_index(&self) -> u8 {
    (self.0 as u8) & ((1 << 6) - 1)
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 8) as *mut _
  }
}
