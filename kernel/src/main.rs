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

use graphics::console;
use graphics::framebuffer::Framebuffer;
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

    let fb = framebuffer_from_limine();
    let fb_info = (fb.width(), fb.height(), fb.bpp());
    console::init(fb);

    // println hits serial and the screen from here on
    println!("RozeOS v0.0.1");
    println!();
    println!("Boot protocol............... Limine");
    println!("Architecture................ x86_64");
    println!("Framebuffer................. {}x{}x{}", fb_info.0, fb_info.1, fb_info.2);
    // copy the geometry out, printing inside with() would reenter the
    // console cell and alias the &mut
    let mut geom = (0, 0);
    console::with(|c| geom = (c.cols(), c.rows()));
    println!("Console..................... {}x{} chars", geom.0, geom.1);
    println!("Kernel...................... initialized");
    println!();
    println!("hello from RozeOS <3");

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

/// stop the world. interrupts are not enabled yet so hlt sleeps forever,
/// the loop is paranoia against spurious wakeups (nmi, smi).
pub fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
