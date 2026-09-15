//! x86_64 bring up: gdt, idt, pic.

pub mod gdt;
pub mod idt;
pub mod pic;
pub mod port;
pub mod power;

/// set up descriptor tables and the interrupt controller. call once,
/// early, before anything that can fault.
pub fn init() {
    gdt::init();
    idt::init();
    pic::init();
}
