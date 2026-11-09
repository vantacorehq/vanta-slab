//! Lock-free slab allocator. Zero runtime dependencies.
//! Fixed-size object pools with atomic free lists.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering};

const OBJECTS_PER_SLAB: usize = 1024;

#[repr(C)]
struct FreeNode {
    next: AtomicPtr<FreeNode>,
}

pub struct Slab {
    size: usize,
    free_list: AtomicPtr<FreeNode>,
    storage: UnsafeCell<[u8; OBJECTS_PER_SLAB * 64]>,
}

unsafe impl Sync for Slab {}

impl Slab {
    pub const fn new(size: usize) -> Self {
        Self {
            size,
            free_list: AtomicPtr::new(ptr::null_mut()),
            storage: UnsafeCell::new([0; OBJECTS_PER_SLAB * 64]),
        }
    }

    /// Call once before use.
    pub fn init(&self) {
        let storage = unsafe { &mut *self.storage.get() };
        let mut prev: *mut FreeNode = ptr::null_mut();

        let node_size = self.size.max(core::mem::size_of::<FreeNode>());
        let step = node_size.next_multiple_of(core::mem::align_of::<FreeNode>());

        for i in (0..OBJECTS_PER_SLAB).rev() {
            let offset = i * step;
            if offset + step > storage.len() {
                break;
            }
            let node_ptr = unsafe { storage.as_mut_ptr().add(offset) as *mut FreeNode };
            unsafe {
                (*node_ptr).next = AtomicPtr::new(prev);
            }
            prev = node_ptr;
        }

        self.free_list.store(prev, Ordering::Release);
    }

    pub fn alloc(&self) -> Option<NonNull<u8>> {
        loop {
            let head = self.free_list.load(Ordering::Acquire);
            if head.is_null() {
                return None;
            }
            let next = unsafe { (*head).next.load(Ordering::Acquire) };
            if self
                .free_list
                .compare_exchange(head, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Some(unsafe { NonNull::new_unchecked(head as *mut u8) });
            }
        }
    }

    pub fn dealloc(&self, ptr: NonNull<u8>) {
        let node = ptr.as_ptr() as *mut FreeNode;
        loop {
            let head = self.free_list.load(Ordering::Acquire);
            unsafe {
                (*node).next.store(head, Ordering::Release);
            }
            if self
                .free_list
                .compare_exchange(head, node, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return;
            }
        }
    }
}

pub struct VantaSlab {
    slabs: [Slab; 7],
}

impl VantaSlab {
    pub const fn new() -> Self {
        Self {
            slabs: [
                Slab::new(16),
                Slab::new(32),
                Slab::new(64),
                Slab::new(128),
                Slab::new(256),
                Slab::new(512),
                Slab::new(1024),
            ],
        }
    }

    pub fn init(&self) {
        for slab in &self.slabs {
            slab.init();
        }
    }

    pub fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size().max(layout.align());
        for slab in &self.slabs {
            if slab.size >= size {
                return slab.alloc();
            }
        }
        None
    }

    pub fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let size = layout.size().max(layout.align());
        for slab in &self.slabs {
            if slab.size >= size {
                slab.dealloc(ptr);
                return;
            }
        }
    }
}

pub struct VantaGlobal(pub VantaSlab);

unsafe impl GlobalAlloc for VantaGlobal {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .allocate(layout)
            .map(|p| p.as_ptr())
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(p) = NonNull::new(ptr) {
            self.0.deallocate(p, layout);
        }
    }
}

            
               
                
      
   
                

   
        
  

   
       

 

