//! framebuffer text console.
//!
//! fixed cell grid over the linear framebuffer using the 8x8 font
//! scaled 2x. wraps at the right edge, scrolls at the bottom, prints
//! through core::fmt so the usual format machinery works. this is the
//! boot log and panic screen until something better exists.

use core::cell::UnsafeCell;
use core::fmt;

use super::font;
use super::framebuffer::{Color, Framebuffer};

/// integer scale applied to the 8x8 glyphs. 2 gives 16 px cells,
/// 80x50 characters at 1280x800.
const SCALE: usize = 2;

/// default palette, light gray on near black.
const FG: Color = 0x00d8_d8d8;
const BG: Color = 0x0018_1818;

/// character cell size in pixels.
const CELL_W: usize = font::GLYPH_W * SCALE;
const CELL_H: usize = font::GLYPH_H * SCALE;

/// text console over a framebuffer. owns the framebuffer, nothing
/// else draws while the console is alive.
pub struct Console {
    fb: Framebuffer,
    cols: usize,
    rows: usize,
    col: usize,
    row: usize,
}

impl Console {
    /// take over a framebuffer, clear it, home the cursor.
    pub fn new(fb: Framebuffer) -> Self {
        let cols = fb.width() / CELL_W;
        let rows = fb.height() / CELL_H;
        let mut c = Self {
            fb,
            cols,
            rows,
            col: 0,
            row: 0,
        };
        c.clear();
        c
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    /// raw framebuffer access for full screen clients like doom.
    /// drawing over console text is the caller's problem.
    pub fn framebuffer(&mut self) -> &mut Framebuffer {
        &mut self.fb
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// wipe the screen and home the cursor.
    pub fn clear(&mut self) {
        self.fb.clear(BG);
        self.col = 0;
        self.row = 0;
    }

    /// draw one glyph cell at the current cursor. background painted
    /// too so overwrites do not smear.
    fn draw_glyph(&mut self, ch: u8) {
        let glyph = &font::FONT[ch as usize];
        let px = self.col * CELL_W;
        let py = self.row * CELL_H;
        for (gy, bits) in glyph.iter().enumerate() {
            for gx in 0..font::GLYPH_W {
                // bit 0 is the leftmost pixel in this font
                let color = if bits & (1 << gx) != 0 { FG } else { BG };
                self.fb.fill_rect(
                    px + gx * SCALE,
                    py + gy * SCALE,
                    SCALE,
                    SCALE,
                    color,
                );
            }
        }
    }

    /// advance to the next line, scrolling when the screen is full.
    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows {
            self.fb.scroll_up(CELL_H, BG);
            self.row = self.rows - 1;
        }
    }

    /// print one byte. handles newline and carriage return, renders
    /// anything non ascii as the 0x7f box.
    pub fn put_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.newline(),
            b'\r' => self.col = 0,
            0x20..=0x7e => {
                self.draw_glyph(b);
                self.col += 1;
                if self.col >= self.cols {
                    self.newline();
                }
            }
            _ => {
                self.draw_glyph(0x7f);
                self.col += 1;
                if self.col >= self.cols {
                    self.newline();
                }
            }
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.put_byte(b);
        }
        Ok(())
    }
}

//=====================================================================
// global console
//=====================================================================

/// wrapper making the global console cell shareable. sound because the
/// kernel is single core and runs with interrupts off, so nothing can
/// observe the cell mid write. revisit when interrupts arrive.
struct ConsoleCell(UnsafeCell<Option<Console>>);

unsafe impl Sync for ConsoleCell {}

static CONSOLE: ConsoleCell = ConsoleCell(UnsafeCell::new(None));

/// install the global console. call once after the framebuffer is up.
pub fn init(fb: Framebuffer) {
    unsafe {
        *CONSOLE.0.get() = Some(Console::new(fb));
    }
}

/// run a closure against the global console if it exists. quietly does
/// nothing before init so early logging only reaches serial.
pub fn with<F: FnOnce(&mut Console)>(f: F) {
    unsafe {
        if let Some(c) = (*CONSOLE.0.get()).as_mut() {
            f(c);
        }
    }
}

/// backend for the print macros: serial always, console once it is up.
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    let _ = crate::serial::Serial.write_fmt(args);
    with(|c| {
        let _ = c.write_fmt(args);
    });
}

/// print to serial and the framebuffer console.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::graphics::console::_print(format_args!($($arg)*))
    };
}

/// print with trailing newline to serial and the framebuffer console.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\n");
    }};
}
