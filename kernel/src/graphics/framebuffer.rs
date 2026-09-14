//! linear framebuffer driver.
//!
//! limine sets the mode through gop/vbe before we run and maps the
//! framebuffer into the higher half. we assume 32 bpp xrgb8888, which
//! is what qemu and everything else this decade actually provides.
//! rows may be padded, hence pitch instead of width * 4.

/// color as 0x00rrggbb. the top byte is ignored by xrgb hardware.
pub type Color = u32;

/// a mapped linear framebuffer and its geometry.
pub struct Framebuffer {
    address: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bpp: usize,
}

impl Framebuffer {
    /// wrap a raw framebuffer mapping.
    ///
    /// safety: address must point at a live framebuffer mapping of at
    /// least height * pitch bytes, and the caller must be the only
    /// writer. single core, no interrupts, so exclusivity holds.
    pub unsafe fn new(
        address: *mut u8,
        width: usize,
        height: usize,
        pitch: usize,
        bpp: usize,
    ) -> Self {
        Self {
            address,
            width,
            height,
            pitch,
            bpp,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bpp(&self) -> usize {
        self.bpp
    }

    /// write one pixel. out of bounds writes are dropped, not wrapped.
    /// idle until doom blits frames through it, keep it around.
    #[allow(dead_code)]
    #[inline]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        // 4 bytes per pixel at 32 bpp. volatile so the compiler cannot
        // fold or reorder writes to what it thinks is dead memory.
        unsafe {
            let p = self.address.add(y * self.pitch + x * 4) as *mut u32;
            core::ptr::write_volatile(p, color);
        }
    }

    /// fill a rectangle. clipped against the screen edges.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        for row in y.min(self.height)..y1 {
            for col in x.min(self.width)..x1 {
                unsafe {
                    let p = self.address.add(row * self.pitch + col * 4) as *mut u32;
                    core::ptr::write_volatile(p, color);
                }
            }
        }
    }

    /// wipe the whole screen to one color.
    pub fn clear(&mut self, color: Color) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }

    /// shift the whole screen up by `px` scanlines and fill the gap at
    /// the bottom. the console uses this for scrolling.
    pub fn scroll_up(&mut self, px: usize, fill: Color) {
        let px = px.min(self.height);
        let moved = self.height - px;
        // overlapping forward copy, dst below src, so plain copy is
        // fine. reading framebuffer memory is slow but correct.
        unsafe {
            core::ptr::copy(
                self.address.add(px * self.pitch),
                self.address,
                moved * self.pitch,
            );
        }
        self.fill_rect(0, moved, self.width, px, fill);
    }
}
