//! idt and exception handlers.
//!
//! the cpu vectors here when something goes wrong. breakpoint returns
//! so it can be used for poking around, everything else prints what it
//! knows and halts. the double fault handler runs on its own ist stack
//! so even a smashed kernel stack produces a readable panic.

use core::cell::UnsafeCell;

use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use super::{gdt, pic};
use crate::input::keyboard;
use crate::println;
use crate::time::timer;

/// shared mutable static, same single core argument as the gdt.
struct Cell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Cell<T> {}

static IDT: Cell<InterruptDescriptorTable> =
    Cell(UnsafeCell::new(InterruptDescriptorTable::new()));

/// fill in the exception vectors and load the table.
pub fn init() {
    unsafe {
        let idt = &mut *IDT.0.get();
        idt.breakpoint.set_handler_fn(breakpoint);
        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.general_protection_fault.set_handler_fn(general_protection);
        idt.page_fault.set_handler_fn(page_fault);
        idt.double_fault
            .set_handler_fn(double_fault)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        // hardware lines, remapped pic base upward
        idt[pic::PIC1_OFFSET].set_handler_fn(timer::on_tick);
        idt[pic::PIC1_OFFSET + 1].set_handler_fn(keyboard::on_irq);
        // static cell, never moves, load_unsafe is fine
        idt.load_unsafe();
    }
}

extern "x86-interrupt" fn breakpoint(frame: InterruptStackFrame) {
    // int3 is the one exception meant to be survived
    println!("breakpoint at {:#x}", frame.instruction_pointer.as_u64());
}

extern "x86-interrupt" fn invalid_opcode(frame: InterruptStackFrame) {
    panic!(
        "invalid opcode at {:#x}",
        frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn general_protection(frame: InterruptStackFrame, code: u64) {
    panic!(
        "general protection fault, code {:#x}, at {:#x}",
        code,
        frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, code: PageFaultErrorCode) {
    // read_raw, a mangled cr2 should not turn the diagnostics into a
    // second panic
    panic!(
        "page fault accessing {:#x}, {:?}, at {:#x}",
        Cr2::read_raw(),
        code,
        frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, _code: u64) -> ! {
    // code is always zero for double faults, nothing lost dropping it
    panic!(
        "double fault, at {:#x}",
        frame.instruction_pointer.as_u64()
    );
}
