//! rozeos kernel entry point.
//!
//! limine hands us control in 64 bit long mode with paging enabled and
//! the kernel mapped at the higher half. no stack juggling, no real mode
//! shims. we set up serial logging and announce ourselves.

#![no_std]
#![no_main]

mod panic;
mod serial;

use limine::BaseRevision;
use limine::request::{RequestsEndMarker, RequestsStartMarker};

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

    halt();
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
