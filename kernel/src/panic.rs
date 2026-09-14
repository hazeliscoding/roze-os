//! kernel panic handler.
//!
//! no unwinding in kernel space, panic means print what we know over
//! serial and halt for good. halting beats the reboot loop a triple
//! fault would give us, the message stays on screen and in the log.

use core::panic::PanicInfo;

use crate::serial_println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!();
    serial_println!("=========================");
    serial_println!("      KERNEL PANIC");
    serial_println!("=========================");
    serial_println!();
    serial_println!("message:");
    serial_println!("{}", info.message());
    serial_println!();
    if let Some(loc) = info.location() {
        serial_println!("location:");
        serial_println!("{}:{}", loc.file(), loc.line());
        serial_println!();
    }
    serial_println!("RozeOS has halted.");

    // interrupts may be on when we get here, kill them before hlt
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
