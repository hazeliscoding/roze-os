//! raw port io. the oldest interface on the machine and still the one
//! the uart, pic and ps/2 controller answer to.

use core::arch::asm;

/// write one byte to an io port.
#[inline]
pub unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val,
             options(nomem, nostack, preserves_flags));
    }
}

/// write one 16 bit word to an io port. acpi pm registers want word
/// writes, byte writes get ignored.
#[inline]
pub unsafe fn outw(port: u16, val: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") val,
             options(nomem, nostack, preserves_flags));
    }
}

/// read one byte from an io port.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!("in al, dx", out("al") val, in("dx") port,
             options(nomem, nostack, preserves_flags));
    }
    val
}

/// stall for roughly a microsecond. a write to port 0x80, the posix of
/// hardware delays. old pics need breathing room between commands.
#[inline]
pub unsafe fn io_wait() {
    unsafe {
        outb(0x80, 0);
    }
}
