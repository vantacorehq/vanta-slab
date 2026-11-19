use std::alloc::{alloc, Layout};
use std::time::Instant;

use vanta_slab::VantaSlab;

fn main() {
    let layout_slab = Layout::new::<VantaSlab>();
    let slab = unsafe {
        let mem = alloc(layout_slab) as *mut VantaSlab;
        assert!(!mem.is_null(), "failed to allocate VantaSlab");
        VantaSlab::write_new(mem);
        Box::from_raw(mem)
    };
    slab.init();
    let layout = Layout::from_size_align(64, 8).expect("layout");
    let iters = 50_000u32;
    let start = Instant::now();
    for _ in 0..iters {
        match slab.allocate(layout) {
            Some(ptr) => slab.deallocate(ptr, layout),
            None => panic!("slab exhausted"),
        }
    }
    let elapsed = start.elapsed();
    println!("vanta-slab alloc/free x{iters}: {elapsed:?}");
}
