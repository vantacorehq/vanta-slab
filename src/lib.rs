//! # Vanta-Slab
//! Lock-free slab allocator. Zero runtime dependencies.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering};

const OBJECTS_PER_SLAB: usize = 512;

struct FreeNode {
    next: AtomicPtr<FreeNode>,
}

pub struct Slab {
    size: usize,
    free_list: AtomicPtr<FreeNode>,
    storage: UnsafeCell<Vec<u8>>, // replaced with fixed in real no_std; keep for simplicity in std context
}

unsafe impl Sync for Slab {}

impl Slab {
    pub fn new(size: usize) -> Self {
        let aligned_size = size.max(core::mem::size_of::<FreeNode>()).next_power_of_two();
        let total = aligned_size * OBJECTS_PER_SLAB;
        let mut storage = vec![0u8; total];
        Self {
            size: aligned_size,
            free_list: AtomicPtr::new(ptr::null_mut()),
            storage: UnsafeCell::new(storage),
        }
    }

    pub fn init(&self) {
        let storage = unsafe { &mut *self.storage.get() };
        let mut prev: *mut FreeNode = ptr::null_mut();

        for i in (0..OBJECTS_PER_SLAB).rev() {
            let offset = i * self.size;
            if offset + self.size > storage.len() {
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
    slabs: [Slab; 6],
}

impl VantaSlab {
    pub fn new() -> Self {
        Self {
            slabs: [
                Slab::new(16),
                Slab::new(32),
                Slab::new(64),
                Slab::new(128),
                Slab::new(256),
                Slab::new(512),
            ],
        }
    }

    pub fn init(&self) {
        for s in &self.slabs {
            s.init();
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
  
           
              
          
   
       

 

