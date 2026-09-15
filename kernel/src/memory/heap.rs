//! kernel heap.
//!
//! a fixed 64 MiB slab of physically contiguous frames, addressed
//! through the higher half direct map so no page table surgery is
//! needed. linked_list_allocator does the bookkeeping and serves the
//! rust alloc machinery: box, vec, string, the lot. doom gets its
//! zone memory from here eventually.

use linked_list_allocator::LockedHeap;

use super::physical::{self, FRAME_SIZE};
use crate::println;

/// heap size. doom's zone allocator, its screen buffer and the level
/// data all live here, 64 MiB leaves headroom to spare.
pub const HEAP_SIZE: usize = 64 * 1024 * 1024;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

/// carve the heap out of physical memory and arm the allocator.
/// call after the frame allocator is up, before any allocation.
pub fn init(hhdm_offset: u64) {
    let frames = HEAP_SIZE / FRAME_SIZE;
    let base = physical::with(|a| a.allocate_contiguous(frames))
        .expect("no contiguous region for the kernel heap");

    // safety: the frames are freshly allocated and unshared, the
    // direct map covers them, and this runs once before any use of
    // the alloc machinery.
    unsafe {
        HEAP.lock()
            .init((hhdm_offset + base) as *mut u8, HEAP_SIZE);
    }

    println!(
        "heap: {} MiB at phys {:#x}",
        HEAP_SIZE / (1024 * 1024),
        base
    );
}
