//! legacy 8259 pic setup.
//!
//! the pair of pics reset mapping irqs 0-7 onto cpu vectors 8-15,
//! straight over the exception range, so a timer tick would look like
//! a double fault. remap them to 32-47 and mask every line. the timer
//! and keyboard milestones unmask what they need.

use super::port::{io_wait, outb};

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xa0;
const PIC2_DATA: u16 = 0xa1;

/// cpu vector where the primary pic lands, 32 = first free after
/// the exception range.
pub const PIC1_OFFSET: u8 = 32;
/// secondary pic vector base.
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

/// remap both pics and mask all irq lines.
pub fn init() {
    unsafe {
        // icw1: start init, expect icw4
        outb(PIC1_CMD, 0x11);
        io_wait();
        outb(PIC2_CMD, 0x11);
        io_wait();
        // icw2: vector offsets
        outb(PIC1_DATA, PIC1_OFFSET);
        io_wait();
        outb(PIC2_DATA, PIC2_OFFSET);
        io_wait();
        // icw3: secondary hangs off irq2 of the primary
        outb(PIC1_DATA, 0x04);
        io_wait();
        outb(PIC2_DATA, 0x02);
        io_wait();
        // icw4: 8086 mode
        outb(PIC1_DATA, 0x01);
        io_wait();
        outb(PIC2_DATA, 0x01);
        io_wait();
        // mask everything until a driver asks for a line
        outb(PIC1_DATA, 0xff);
        outb(PIC2_DATA, 0xff);
    }
}
