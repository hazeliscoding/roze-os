//! pit driver, channel 0.
//!
//! the 8253/8254 runs at 1193182 Hz. programmed as a rate generator
//! with divisor 1193 it fires irq0 every 0.99985 ms, close enough to
//! call a millisecond. the handler bumps an atomic counter, sleep
//! spins on it with hlt so the cpu naps between ticks.

use core::sync::atomic::{AtomicU64, Ordering};

use x86_64::structures::idt::InterruptStackFrame;

use crate::arch::x86_64::pic;
use crate::arch::x86_64::port::outb;

/// tick rate. one tick, one millisecond, near enough.
pub const HZ: u32 = 1000;

/// pit input clock in Hz, fixed by the hardware since 1981.
const PIT_CLOCK: u32 = 1_193_182;

const PIT_CH0: u16 = 0x40;
const PIT_CMD: u16 = 0x43;

/// milliseconds since the pit came up. wraps after 584 million
/// years, acceptable technical debt.
static TICKS: AtomicU64 = AtomicU64::new(0);

/// program the pit and open its irq line. interrupts still need a
/// global sti after this.
pub fn init() {
    let divisor = (PIT_CLOCK / HZ) as u16;
    unsafe {
        // channel 0, lobyte/hibyte access, mode 2 rate generator
        outb(PIT_CMD, 0x34);
        outb(PIT_CH0, divisor as u8);
        outb(PIT_CH0, (divisor >> 8) as u8);
    }
    pic::unmask(0);
}

/// irq0 handler, wired into the idt at vector 32.
pub extern "x86-interrupt" fn on_tick(_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    pic::eoi(0);
}

/// milliseconds since boot, give or take pit drift.
pub fn ticks_ms() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// busy wait, but hlt between ticks so the busy part is a nap. only
/// meaningful after interrupts are enabled, otherwise it never wakes.
pub fn sleep_ms(ms: u64) {
    let end = ticks_ms() + ms;
    while ticks_ms() < end {
        x86_64::instructions::hlt();
    }
}
