//! reboot and shutdown.
//!
//! reboot pulses the 8042 reset line, which has owned the reset wire
//! since the ibm at. shutdown writes s5 sleep to the qemu q35 acpi
//! pm1a control register. neither should return, both fall back to a
//! halt loop if the hardware shrugs.

use super::port::{inb, outb, outw};

/// pulse the ps/2 controller reset line. the canonical soft reboot
/// on pc hardware, qemu included.
pub fn reboot() -> ! {
    unsafe {
        // drain the 8042 input buffer so the command is not eaten
        for _ in 0..1024 {
            if inb(0x64) & 2 == 0 {
                break;
            }
        }
        outb(0x64, 0xfe);
    }
    halt_forever();
}

/// power off the machine. slp_typ 0 + slp_en is s5 on qemu's ich9,
/// the i440fx port is written too in case the machine type changes.
pub fn shutdown() -> ! {
    unsafe {
        outw(0x604, 0x2000); // q35 / ich9
        outw(0xb004, 0x2000); // i440fx, harmless on q35
    }
    halt_forever();
}

/// the request was ignored, park the cpu instead of returning into a
/// menu that thinks the machine is going away.
fn halt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
