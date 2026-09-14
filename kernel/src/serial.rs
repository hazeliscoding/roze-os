//! 16550 uart driver for com1.
//!
//! qemu wires com1 to whatever -serial points at, which makes this the
//! debug channel of record. polled io only, no interrupts. the kernel is
//! single core with interrupts off, so unguarded port access is sound.

use core::fmt;

use crate::arch::x86_64::port::{inb, outb};

/// com1 base port. pc compatible since the ibm at.
const COM1: u16 = 0x3f8;

// register offsets from base. dlab bit in lcr banks the first two.
const DATA: u16 = 0; // thr/rbr, or divisor low with dlab
const IER: u16 = 1; // interrupt enable, or divisor high with dlab
const FCR: u16 = 2; // fifo control
const LCR: u16 = 3; // line control
const MCR: u16 = 4; // modem control
const LSR: u16 = 5; // line status

/// lsr bit 5, transmit holding register empty.
const LSR_THRE: u8 = 1 << 5;

/// program com1 for 115200 8n1 with fifos on. call once at boot before
/// any logging happens.
pub fn init() {
    unsafe {
        outb(COM1 + IER, 0x00); // no interrupts, we poll
        outb(COM1 + LCR, 0x80); // dlab on to reach the divisor
        outb(COM1 + DATA, 0x01); // divisor 1 = 115200 baud
        outb(COM1 + IER, 0x00);
        outb(COM1 + LCR, 0x03); // 8n1, dlab off
        outb(COM1 + FCR, 0xc7); // fifo on, clear both, 14 byte trigger
        outb(COM1 + MCR, 0x03); // dtr + rts up
    }
}

/// spin until the uart can take a byte, then hand it over.
fn write_byte(b: u8) {
    unsafe {
        while inb(COM1 + LSR) & LSR_THRE == 0 {}
        outb(COM1 + DATA, b);
    }
}

/// zero sized handle implementing core::fmt::Write against com1.
/// construct freely, there is no state to race on.
pub struct Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            // terminals want crlf, rust strings carry bare lf
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
        Ok(())
    }
}

/// print to com1. same shape as std print.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::serial::Serial, $($arg)*);
    }};
}

/// print to com1 with trailing newline.
#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => {{
        $crate::serial_print!($($arg)*);
        $crate::serial_print!("\n");
    }};
}
