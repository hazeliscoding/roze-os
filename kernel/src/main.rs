//! rozeos kernel entry point.
//!
//! limine hands us control in 64 bit long mode with paging enabled and
//! the kernel mapped at the higher half. no stack juggling, no real mode
//! shims. we set up serial logging and announce ourselves.

#![no_std]
#![no_main]

mod graphics;
mod panic;
mod serial;

use graphics::framebuffer::{self, Framebuffer};
use limine::BaseRevision;
use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};

/// tells limine which protocol revision we speak. the bootloader patches
/// this in place, is_supported() reads the result.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

/// request section fence, start. limine only scans between the markers.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

/// request section fence, end.
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

/// ask limine for a linear framebuffer. it picks the native mode.
#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// kernel entry. named in linker.ld as the elf entry point.
#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    serial::init();

    if !BASE_REVISION.is_supported() {
        panic!("limine base revision not supported by bootloader");
    }

    serial_println!("RozeOS v0.0.1");
    serial_println!();
    serial_println!("Boot protocol............... Limine");
    serial_println!("Architecture................ x86_64");
    serial_println!("Kernel...................... initialized");
    serial_println!();
    serial_println!("hello from RozeOS <3");

    let mut fb = framebuffer_from_limine();
    serial_println!(
        "framebuffer {}x{}x{}",
        fb.width(),
        fb.height(),
        fb.bpp()
    );
    test_screen(&mut fb);
    serial_println!("test screen drawn");

    halt();
}

/// pull the first framebuffer out of the limine response and wrap it.
fn framebuffer_from_limine() -> Framebuffer {
    let resp = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("limine gave no framebuffer response");
    let fb = resp
        .framebuffers()
        .next()
        .expect("limine response holds no framebuffers");
    // limine guarantees the mapping is live and covers height * pitch.
    // nothing else draws, so we own it.
    unsafe {
        Framebuffer::new(
            fb.addr(),
            fb.width() as usize,
            fb.height() as usize,
            fb.pitch() as usize,
            fb.bpp() as usize,
        )
    }
}

/// milestone 2 test pattern: dark field, white border, color bars up
/// top, grayscale ramp below them. wrong colors here mean the pixel
/// format assumption is off, wrong geometry means pitch handling is.
fn test_screen(fb: &mut Framebuffer) {
    let w = fb.width();
    let h = fb.height();

    fb.clear(0x0018_1818);

    // one pixel white frame hugging the edges, catches off by ones
    fb.fill_rect(0, 0, w, 1, framebuffer::WHITE);
    fb.fill_rect(0, h - 1, w, 1, framebuffer::WHITE);
    fb.fill_rect(0, 0, 1, h, framebuffer::WHITE);
    fb.fill_rect(w - 1, 0, 1, h, framebuffer::WHITE);

    // color bars, left to right: white red green blue cyan magenta
    // yellow. primaries out of order mean channel swap.
    let bars: [framebuffer::Color; 7] = [
        0x00ff_ffff,
        0x00ff_0000,
        0x0000_ff00,
        0x0000_00ff,
        0x0000_ffff,
        0x00ff_00ff,
        0x00ff_ff00,
    ];
    let bar_w = (w - 32) / bars.len();
    for (i, &c) in bars.iter().enumerate() {
        fb.fill_rect(16 + i * bar_w, 16, bar_w - 8, h / 4, c);
    }

    // grayscale ramp under the bars, 32 steps black to white
    let steps = 32;
    let step_w = (w - 32) / steps;
    for i in 0..steps {
        let v = (i * 255 / (steps - 1)) as u32;
        let c = (v << 16) | (v << 8) | v;
        fb.fill_rect(16 + i * step_w, 32 + h / 4, step_w, h / 8, c);
    }

    // centered rectangle where doom will live one day
    let dw = w / 2;
    let dh = h / 4;
    let dx = (w - dw) / 2;
    let dy = h / 2 + h / 8;
    fb.fill_rect(dx, dy, dw, dh, 0x0080_2040);

    // per pixel diagonals across it, exercises put_pixel and shows
    // tearing if pitch math ever goes sideways
    for i in 0..dw {
        let y = dy + (i * dh) / dw;
        fb.put_pixel(dx + i, y, framebuffer::WHITE);
        fb.put_pixel(dx + dw - 1 - i, y, framebuffer::WHITE);
    }
}

/// stop the world. interrupts are not enabled yet so hlt sleeps forever,
/// the loop is paranoia against spurious wakeups (nmi, smi).
pub fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
