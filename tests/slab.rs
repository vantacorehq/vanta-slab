use std::alloc::{alloc, Layout as AllocLayout};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use vanta_slab::{Slab, VantaSlab};

fn boxed_slab(size: usize) -> Box<Slab> {
    Box::new(Slab::new(size))
}

fn boxed_vanta() -> Box<VantaSlab> {
    let layout = AllocLayout::new::<VantaSlab>();
    unsafe {
        let mem = alloc(layout) as *mut VantaSlab;
        assert!(!mem.is_null(), "failed to allocate VantaSlab");
        VantaSlab::write_new(mem);
        Box::from_raw(mem)
    }
}

#[test]
fn init_then_alloc_returns_non_null() {
    let slab = boxed_slab(32);
    slab.init();
    assert!(!slab.is_empty(), "init should publish a free list");
    let ptr = slab.alloc().expect("alloc after init");
    assert_ne!(ptr.as_ptr() as usize, 0);

    let multi = boxed_vanta();
    multi.init();
    let layout = std::alloc::Layout::from_size_align(64, 8).unwrap();
    let got = multi.allocate(layout).expect("size-class alloc");
    assert_ne!(got.as_ptr() as usize, 0);
}

#[test]
fn dealloc_then_alloc_reuses_the_block() {
    let slab = boxed_slab(32);
    slab.init();
    let first = slab.alloc().unwrap();
    let second = slab.alloc().unwrap();
    assert_ne!(first.as_ptr(), second.as_ptr());
    slab.dealloc(first);
    let reused = slab.alloc().expect("alloc after dealloc");
    assert_eq!(reused.as_ptr(), first.as_ptr());
}

#[test]
fn allocating_until_empty_returns_none() {
    // 2048-byte blocks cannot fill 1024 slots in a 1024 * 256 byte buffer.
    let slab = boxed_slab(2048);
    slab.init();
    let mut n = 0usize;
    while slab.alloc().is_some() {
        n += 1;
        assert!(n < 1024, "2048-byte class must not hold 1024 blocks");
    }
    assert_eq!(n, 128, "expected 262144 / 2048 = 128 blocks, got {n}");
    assert!(slab.alloc().is_none());
    assert!(slab.is_empty());
}

#[test]
fn backing_buffer_limits_larger_classes() {
    let cases = [
        (16usize, 1024usize),
        (256, 1024),
        (512, 512),
        (1024, 256),
        (2048, 128),
    ];
    for (size, expect) in cases {
        let slab = boxed_slab(size);
        slab.init();
        let mut n = 0usize;
        while slab.alloc().is_some() {
            n += 1;
            if n > expect + 1 {
                break;
            }
        }
        assert_eq!(n, expect, "size class {size}");
    }
}

#[test]
fn two_threads_do_not_receive_the_same_pointer() {
    let slab = Arc::new(Slab::new(64));
    slab.init();
    let mut handles = Vec::new();
    for _ in 0..2 {
        let slab = Arc::clone(&slab);
        handles.push(thread::spawn(move || {
            let mut ptrs = Vec::with_capacity(300);
            for _ in 0..300 {
                let ptr = slab.alloc().expect("slab should have free blocks");
                ptrs.push(ptr.as_ptr() as usize);
            }
            ptrs
        }));
    }
    let mut all = Vec::new();
    for handle in handles {
        all.extend(handle.join().expect("thread"));
    }
    let unique: HashSet<usize> = all.iter().copied().collect();
    assert_eq!(all.len(), 600);
    assert_eq!(
        unique.len(),
        all.len(),
        "racing allocs returned a duplicate pointer"
    );
}
