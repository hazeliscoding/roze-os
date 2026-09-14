//! physical frame allocator.
//!
//! one bit per 4 KiB frame over everything up to the highest usable
//! address, set means used. the bitmap itself lives in stolen frames
//! from the first usable region big enough to hold it, reached through
//! the limine higher half direct map. linear scan with a rolling hint,
//! nobody needs better until doom says otherwise.

use core::cell::UnsafeCell;

use crate::println;

/// frame size, the only page size this kernel knows.
pub const FRAME_SIZE: usize = 4096;

/// a usable region of physical memory, plain numbers so this module
/// does not depend on limine types.
#[derive(Clone, Copy)]
pub struct Region {
    pub base: u64,
    pub length: u64,
}

/// bitmap allocator state.
pub struct FrameAllocator {
    /// one bit per frame, set = used. lives in the direct map.
    bitmap: &'static mut [u8],
    /// total frames tracked, bitmap length in bits.
    frames: usize,
    /// scan hint, index of the last allocation.
    next: usize,
    /// free frames remaining.
    free: usize,
}

impl FrameAllocator {
    #[inline]
    fn set(&mut self, i: usize) {
        self.bitmap[i / 8] |= 1 << (i % 8);
    }

    #[inline]
    fn clear(&mut self, i: usize) {
        self.bitmap[i / 8] &= !(1 << (i % 8));
    }

    #[inline]
    fn get(&self, i: usize) -> bool {
        self.bitmap[i / 8] & (1 << (i % 8)) != 0
    }

    /// grab a free frame, returns its physical address.
    pub fn allocate_frame(&mut self) -> Option<u64> {
        // scan from the hint, wrap once
        for off in 0..self.frames {
            let i = (self.next + off) % self.frames;
            if !self.get(i) {
                self.set(i);
                self.next = i + 1;
                self.free -= 1;
                return Some((i * FRAME_SIZE) as u64);
            }
        }
        None
    }

    /// return a frame. freeing a frame that is not allocated is a bug
    /// in the caller, loud beats corrupt.
    pub fn free_frame(&mut self, addr: u64) {
        assert!(addr as usize % FRAME_SIZE == 0, "unaligned frame address");
        let i = addr as usize / FRAME_SIZE;
        assert!(i < self.frames, "frame address out of range");
        assert!(self.get(i), "double free of frame {:#x}", addr);
        self.clear(i);
        self.free += 1;
    }

    /// free frames remaining.
    pub fn free_frames(&self) -> usize {
        self.free
    }

    /// grab `count` physically contiguous frames, returns the base
    /// address. the heap wants one flat run inside the direct map.
    /// dumb forward scan, runs once at boot, speed is irrelevant.
    pub fn allocate_contiguous(&mut self, count: usize) -> Option<u64> {
        let mut run = 0;
        for i in 0..self.frames {
            if self.get(i) {
                run = 0;
                continue;
            }
            run += 1;
            if run == count {
                let first = i + 1 - count;
                for j in first..=i {
                    self.set(j);
                }
                self.free -= count;
                return Some((first * FRAME_SIZE) as u64);
            }
        }
        None
    }
}

//=====================================================================
// global allocator instance
//=====================================================================

/// single core, interrupts masked, init once: the usual argument.
struct Cell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Cell<T> {}

static ALLOCATOR: Cell<Option<FrameAllocator>> = Cell(UnsafeCell::new(None));

/// run a closure against the frame allocator. panics before init,
/// allocating frames that early is a bug.
pub fn with<R>(f: impl FnOnce(&mut FrameAllocator) -> R) -> R {
    unsafe {
        let a = (*ALLOCATOR.0.get())
            .as_mut()
            .expect("frame allocator used before init");
        f(a)
    }
}

/// build the allocator from the limine memory map.
///
/// `usable` holds the usable regions, `hhdm_offset` is the virtual
/// base of the direct map so we can write the bitmap into physical
/// memory we picked ourselves.
pub fn init(usable: &[Region], hhdm_offset: u64) {
    // frames tracked: up to the end of the highest usable region.
    // reserved holes stay marked used and never move.
    let top = usable
        .iter()
        .map(|r| r.base + r.length)
        .max()
        .expect("memory map holds no usable regions");
    let frames = (top as usize).div_ceil(FRAME_SIZE);
    let bitmap_bytes = frames.div_ceil(8);

    // steal the bitmap from the first region that fits it
    let home = usable
        .iter()
        .find(|r| r.length as usize >= bitmap_bytes)
        .expect("no usable region large enough for the frame bitmap");

    // safety: the region is usable ram, the direct map covers it, and
    // nothing else owns memory yet. exclusive by construction.
    let bitmap: &'static mut [u8] = unsafe {
        core::slice::from_raw_parts_mut((hhdm_offset + home.base) as *mut u8, bitmap_bytes)
    };

    // all used, then punch out the usable regions
    bitmap.fill(0xff);
    let mut alloc = FrameAllocator {
        bitmap,
        frames,
        next: 0,
        free: 0,
    };
    for r in usable {
        let first = (r.base as usize).div_ceil(FRAME_SIZE);
        let last = (r.base + r.length) as usize / FRAME_SIZE;
        for i in first..last {
            alloc.clear(i);
            alloc.free += 1;
        }
    }

    // the bitmap frames themselves are spoken for
    let first = home.base as usize / FRAME_SIZE;
    let last = (home.base as usize + bitmap_bytes).div_ceil(FRAME_SIZE);
    for i in first..last {
        if !alloc.get(i) {
            alloc.set(i);
            alloc.free -= 1;
        }
    }

    println!(
        "physical memory: {} MiB usable, {} frames tracked, bitmap {} KiB",
        alloc.free * FRAME_SIZE / (1024 * 1024),
        alloc.frames,
        bitmap_bytes / 1024
    );

    unsafe {
        *ALLOCATOR.0.get() = Some(alloc);
    }
}
