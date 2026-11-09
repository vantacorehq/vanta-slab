# vanta-slab

Lock-free slab allocator written in pure Rust.

Implements fixed-size object pools using atomic free lists.  
No external dependencies. Compatible with `no_std`.

## What it does

- Allocates and frees fixed-size blocks from pre-allocated slabs
- Uses compare-and-swap on free lists
- Supports sizes from 16 to 2048 bytes

## Usage

```rust
use vanta_slab::VantaSlab;
use std::alloc::Layout;

let slab = VantaSlab::new();
slab.init();

let layout = Layout::from_size_align(64, 8).unwrap();
if let Some(ptr) = slab.allocate(layout) {
    // ...
    slab.deallocate(ptr, layout);
}
