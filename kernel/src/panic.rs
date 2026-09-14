//! kernel panic handler.
//!
//! no unwinding in kernel space, panic means print what we know and
//! halt for good. println hits serial always and the framebuffer
//! console once it is up, so panics are visible on screen too.
//! halting beats the reboot loop a triple fault would give us.

use core::panic::PanicInfo;

use crate::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!();
    println!("=========================");
    println!("      KERNEL PANIC");
    println!("=========================");
    println!();
    println!("message:");
    println!("{}", info.message());
    println!();
    if let Some(loc) = info.location() {
        println!("location:");
        println!("{}:{}", loc.file(), loc.line());
        println!();
    }
    println!("RozeOS has halted.");

    // interrupts may be on when we get here, kill them before hlt
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
