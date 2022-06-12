
use std::{
  intrinsics::transmute,
  sync::atomic::{AtomicU64, Ordering, fence},
  ptr::null_mut,
  alloc::{Layout, alloc}};


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
const OrphanPage : PageHeaderData =
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
        slab_ptr.cast(), 1);
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
    loop {
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
        Ok(_) => break,
        Err(actual) => page_header_ = actual,
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
        slab_ptr, free_slab_index as u8);
    return control_item;
  }
  pub fn release_memory(
    &mut self, control_item: MemorySlabControlItem
  ) { unsafe {
    let index = 1 << control_item.project_index();
    let ptr = control_item.project_ptr();
    let header = &mut *ptr.cast::<AtomicU64>();
    let previous = header.fetch_xor(index, Ordering::Relaxed);
    if previous ^ index == transmute(OrphanPage) { // hell, yeah! free page
      *ptr.cast::<*mut ()>() = self.free_page_list;
      self.free_page_list = ptr;
    }
  } }
}

#[derive(Debug, Clone, Copy)]
pub struct MemorySlabControlItem(u64);
impl MemorySlabControlItem {
  pub fn init_null() -> Self { Self(0) }
  pub fn is_null(&self) -> bool { self.0 == 0 }
  pub fn init(slab_ptr: *mut (), index: u8) -> Self {
    let a = ((slab_ptr as u64) << 8) + index as u64;
    Self(a)
  }
  pub fn project_index(&self) -> u8 {
    unsafe { transmute(self.0 as u8) }
  }
  pub fn project_ptr(&self) -> *mut () {
    (self.0 >> 8) as *mut _
  }
}
