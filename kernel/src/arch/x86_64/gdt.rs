//! gdt and tss.
//!
//! long mode barely uses segmentation but the cpu still demands a gdt
//! with proper code and data descriptors, and the tss is the only way
//! to get a known good stack for the double fault handler. without the
//! ist stack a kernel stack overflow would double fault onto the same
//! dead stack and triple fault into a silent reboot.

use core::cell::UnsafeCell;

use x86_64::VirtAddr;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, DS, ES, SS, Segment};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;

/// ist slot used by the double fault handler.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// dedicated double fault stack. 20 KiB, no guard page, good enough
/// until real memory management exists.
const IST_STACK_SIZE: usize = 4096 * 5;

/// shared mutable statics. sound because init runs once on the single
/// core with interrupts off, and nothing writes after that.
struct Cell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Cell<T> {}

static DF_STACK: Cell<[u8; IST_STACK_SIZE]> = Cell(UnsafeCell::new([0; IST_STACK_SIZE]));
static TSS: Cell<TaskStateSegment> = Cell(UnsafeCell::new(TaskStateSegment::new()));
static GDT: Cell<GlobalDescriptorTable> = Cell(UnsafeCell::new(GlobalDescriptorTable::new()));

/// build and load the gdt, reload segment registers, load the tss.
pub fn init() {
    unsafe {
        // stacks grow down, hand the tss the top of the region
        let stack_top = VirtAddr::from_ptr(DF_STACK.0.get()) + IST_STACK_SIZE as u64;
        let tss = &mut *TSS.0.get();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_top;

        let gdt = &mut *GDT.0.get();
        let code = gdt.append(Descriptor::kernel_code_segment());
        let data = gdt.append(Descriptor::kernel_data_segment());
        let tss_sel = gdt.append(Descriptor::tss_segment_unchecked(TSS.0.get()));

        // the table lives in a static cell, it is never moved or freed
        gdt.load_unsafe();

        CS::set_reg(code);
        SS::set_reg(data);
        DS::set_reg(data);
        ES::set_reg(data);
        load_tss(tss_sel);
    }
}
