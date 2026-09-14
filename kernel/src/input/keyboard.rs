//! ps/2 keyboard driver.
//!
//! irq1 fires, we read one scancode byte from port 0x60 and drop it
//! in a ring buffer. that is the whole interrupt handler. decoding
//! happens at the consumer end: scancode set 1, make/break in bit 7,
//! 0xe0 prefix for the extended keys like arrows.
//!
//! the pipeline from the plan:
//!   irq -> scancode -> decoder -> key event queue -> consumer

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use x86_64::structures::idt::InterruptStackFrame;

use crate::arch::x86_64::pic;
use crate::arch::x86_64::port::inb;

/// ps/2 data port.
const DATA_PORT: u16 = 0x60;

/// keyboard irq line on the primary pic.
const IRQ: u8 = 1;

//=====================================================================
// scancode ring buffer
//=====================================================================

/// power of two so wraparound is a mask.
const QUEUE_SIZE: usize = 256;

struct Queue {
    buf: UnsafeCell<[u8; QUEUE_SIZE]>,
    /// write index, owned by the irq handler.
    head: AtomicUsize,
    /// read index, owned by the consumer.
    tail: AtomicUsize,
}

// single producer (irq handler), single consumer (kernel main loop),
// indices are atomics with acquire/release pairing. sound on one core
// and still sound if this ever grows a second one.
unsafe impl Sync for Queue {}

static QUEUE: Queue = Queue {
    buf: UnsafeCell::new([0; QUEUE_SIZE]),
    head: AtomicUsize::new(0),
    tail: AtomicUsize::new(0),
};

impl Queue {
    /// called from the irq handler. full queue drops the byte, losing
    /// a keystroke beats deadlocking an interrupt.
    fn push(&self, b: u8) {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= QUEUE_SIZE {
            return;
        }
        unsafe {
            (*self.buf.get())[head % QUEUE_SIZE] = b;
        }
        self.head.store(head.wrapping_add(1), Ordering::Release);
    }

    /// called from the consumer.
    fn pop(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let b = unsafe { (*self.buf.get())[tail % QUEUE_SIZE] };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(b)
    }
}

//=====================================================================
// scancode decoding
//=====================================================================

/// the keys doom cares about plus enough alphabet to type a name in
/// the high score table that does not exist yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyCode {
    Escape,
    Enter,
    Space,
    LCtrl,
    LShift,
    RShift,
    LAlt,
    Up,
    Down,
    Left,
    Right,
    /// ascii letter or digit, uppercase.
    Char(u8),
    /// something we do not map yet, raw scancode preserved.
    Unknown(u8),
}

/// one key transition.
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub pressed: bool,
}

/// set 1 decode for the base table, make codes only.
fn decode_base(sc: u8) -> KeyCode {
    match sc {
        0x01 => KeyCode::Escape,
        0x1c => KeyCode::Enter,
        0x1d => KeyCode::LCtrl,
        0x2a => KeyCode::LShift,
        0x36 => KeyCode::RShift,
        0x38 => KeyCode::LAlt,
        0x39 => KeyCode::Space,
        // number row, 0x0b is zero because the row starts at one
        0x02..=0x0a => KeyCode::Char(b'1' + sc - 0x02),
        0x0b => KeyCode::Char(b'0'),
        // letter rows in qwerty layout order
        0x10..=0x19 => KeyCode::Char(b"QWERTYUIOP"[(sc - 0x10) as usize]),
        0x1e..=0x26 => KeyCode::Char(b"ASDFGHJKL"[(sc - 0x1e) as usize]),
        0x2c..=0x32 => KeyCode::Char(b"ZXCVBNM"[(sc - 0x2c) as usize]),
        other => KeyCode::Unknown(other),
    }
}

/// set 1 decode for the 0xe0 extended table.
fn decode_extended(sc: u8) -> KeyCode {
    match sc {
        0x48 => KeyCode::Up,
        0x50 => KeyCode::Down,
        0x4b => KeyCode::Left,
        0x4d => KeyCode::Right,
        0x1d => KeyCode::LCtrl, // right ctrl, doom does not care which
        other => KeyCode::Unknown(other),
    }
}

/// decoder state, one byte of it. e0 remembers a pending extended
/// prefix between feed calls.
static E0_PENDING: AtomicUsize = AtomicUsize::new(0);

/// pull the next key event out of the queue, if any. run this from
/// the main loop, not from interrupt context.
pub fn poll_event() -> Option<KeyEvent> {
    while let Some(sc) = QUEUE.pop() {
        if sc == 0xe0 {
            E0_PENDING.store(1, Ordering::Relaxed);
            continue;
        }
        let extended = E0_PENDING.swap(0, Ordering::Relaxed) != 0;
        let pressed = sc & 0x80 == 0;
        let base = sc & 0x7f;
        let code = if extended {
            decode_extended(base)
        } else {
            decode_base(base)
        };
        return Some(KeyEvent { code, pressed });
    }
    None
}

//=====================================================================
// init and irq
//=====================================================================

/// drain the controller and open the irq line. qemu boots the ps/2
/// controller in translated set 1 with scanning on, no setup dance.
pub fn init() {
    unsafe {
        // flush anything stale so the first real key is not garbage
        while inb(0x64) & 1 != 0 {
            let _ = inb(DATA_PORT);
        }
    }
    pic::unmask(IRQ);
}

/// irq1 handler, wired at vector 33. read the byte, queue it, eoi.
pub extern "x86-interrupt" fn on_irq(_frame: InterruptStackFrame) {
    let sc = unsafe { inb(DATA_PORT) };
    QUEUE.push(sc);
    pic::eoi(IRQ);
}
