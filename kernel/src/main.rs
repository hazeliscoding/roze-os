//! rozeos kernel entry point.
//!
//! limine hands us control in 64 bit long mode with paging enabled and
//! the kernel mapped at the higher half. no stack juggling, no real mode
//! shims. we set up serial logging and announce ourselves.

#![no_std]
#![no_main]
// exception handlers use the interrupt calling convention, nightly only
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod doom;
mod graphics;
mod input;
mod memory;
mod menu;
mod panic;
mod serial;
mod time;

use graphics::console;
use graphics::framebuffer::Framebuffer;
use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RequestsEndMarker,
    RequestsStartMarker,
};

/// tells limine which protocol revision we speak. the bootloader patches
/// this in place, is_supported() reads the result.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

/// request section fence, start. limine only scans between the markers.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

/// request section fence, end.
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

/// ask limine for a linear framebuffer. it picks the native mode.
#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// physical memory map, the ground truth for the frame allocator.
#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// higher half direct map offset. physical address x lives at
/// hhdm_offset + x, which is how the kernel touches raw ram.
#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

/// boot modules. the wad rides in as one, no filesystem needed.
#[used]
#[unsafe(link_section = ".requests")]
static MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

/// kernel entry. named in linker.ld as the elf entry point.
#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    serial::init();

    if !BASE_REVISION.is_supported() {
        panic!("limine base revision not supported by bootloader");
    }

    // tables first, so anything that faults from here on panics
    // readable instead of triple faulting
    arch::x86_64::init();

    let hhdm = init_physical_memory();
    memory::heap::init(hhdm);

    // clock and keyboard on, then let the world in
    time::timer::init();
    input::keyboard::init();
    x86_64::instructions::interrupts::enable();

    let fb = framebuffer_from_limine();
    let fb_info = (fb.width(), fb.height(), fb.bpp());
    console::init(fb);

    // println hits serial and the screen from here on
    println!("RozeOS v0.0.1");
    println!();
    println!("Boot protocol............... Limine");
    println!("Architecture................ x86_64");
    println!("Framebuffer................. {}x{}x{}", fb_info.0, fb_info.1, fb_info.2);
    let free_mib = memory::physical::with(|a| {
        a.free_frames() * memory::physical::FRAME_SIZE / (1024 * 1024)
    });
    println!("Memory...................... {} MiB free", free_mib);
    // copy the geometry out, printing inside with() would reenter the
    // console cell and alias the &mut
    let mut geom = (0, 0);
    console::with(|c| geom = (c.cols(), c.rows()));
    println!("Console..................... {}x{} chars", geom.0, geom.1);
    println!("Interrupts.................. GDT, IDT, PIC remapped");
    println!("Timer....................... PIT at {} Hz", time::timer::HZ);
    println!("Keyboard.................... PS/2, IRQ1");
    println!("Kernel...................... initialized");
    println!();
    println!("hello from RozeOS <3");

    // boot self test: int3 must come back alive. proves the idt is
    // loaded and handlers return properly.
    x86_64::instructions::interrupts::int3();
    println!("int3 handled, back in kmain");

    frame_allocator_self_test();
    heap_self_test();
    timer_self_test();
    c_interop_self_test();

    // the menu owns the machine from here. with a wad on board doom
    // is one keypress away, without one the entry sits parked.
    let wad = find_wad();
    match wad {
        Some(w) => println!("wad module found, {} bytes", w.len()),
        None => println!("no wad module, doom entry disabled"),
    }
    menu::run(wad);
}

/// find the wad among the boot modules by file extension.
fn find_wad() -> Option<&'static [u8]> {
    let resp = MODULE_REQUEST.get_response()?;
    for module in resp.modules() {
        let path = module.path().to_bytes();
        if path.len() >= 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".wad") {
            // limine keeps modules mapped for the kernel's lifetime
            let data = unsafe {
                core::slice::from_raw_parts(module.addr(), module.size() as usize)
            };
            return Some(data);
        }
    }
    None
}

// compiled c, see kernel/cbits and build.rs
unsafe extern "C" {
    fn roze_test() -> i32;
}

/// boot self test: call into compiled c and check the answer. proves
/// the whole clang to rust-lld pipeline doomgeneric will ride.
fn c_interop_self_test() {
    let r = unsafe { roze_test() };
    assert!(r == 42, "roze_test returned {r}, expected 42");
    println!("c interop self test passed, roze_test() = {r}");
}

/// boot self test: sleep 100 ms and check the clock moved a sane
/// amount. qemu timing is loose, the window is generous.
fn timer_self_test() {
    let t0 = time::timer::ticks_ms();
    time::timer::sleep_ms(100);
    let elapsed = time::timer::ticks_ms() - t0;
    assert!(
        (90..=400).contains(&elapsed),
        "slept 100 ms but measured {} ms",
        elapsed
    );
    println!("timer self test passed, 100 ms sleep measured {} ms", elapsed);
}

/// boot self test: the alloc machinery must actually work. box, vec,
/// string, growth past the first allocation, and drop.
fn heap_self_test() {
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;

    let b = Box::new(0xc0ffee_u32);
    let mut v: Vec<u64> = Vec::new();
    for i in 0..1000 {
        v.push(i);
    }
    let mut s = String::from("heap self test");
    s.push_str(" passed");
    assert!(*b == 0xc0ffee, "box roundtrip failed");
    assert!(v.len() == 1000 && v[999] == 999, "vec growth failed");
    drop(v);
    println!("{} ({} bytes boxed at {:p})", s, core::mem::size_of::<u32>(), b);
}

/// feed the limine memory map into the frame allocator. returns the
/// hhdm offset for whoever needs the direct map next.
fn init_physical_memory() -> u64 {
    let hhdm = HHDM_REQUEST
        .get_response()
        .expect("limine gave no hhdm response")
        .offset();
    let map = MEMORY_MAP_REQUEST
        .get_response()
        .expect("limine gave no memory map");

    // collect usable regions into a fixed buffer, no heap this early
    let mut usable = [memory::physical::Region { base: 0, length: 0 }; 64];
    let mut n = 0;
    for entry in map.entries() {
        if entry.entry_type == limine::memory_map::EntryType::USABLE {
            assert!(n < usable.len(), "more usable regions than expected");
            usable[n] = memory::physical::Region {
                base: entry.base,
                length: entry.length,
            };
            n += 1;
        }
    }
    memory::physical::init(&usable[..n], hhdm);
    hhdm
}

/// boot self test: two allocations must hand out distinct aligned
/// frames, a freed frame must come back, the free count must balance.
fn frame_allocator_self_test() {
    memory::physical::with(|a| {
        let before = a.free_frames();
        let f1 = a.allocate_frame().expect("first frame allocation failed");
        let f2 = a.allocate_frame().expect("second frame allocation failed");
        assert!(f1 != f2, "allocator handed out the same frame twice");
        assert!(f1 % memory::physical::FRAME_SIZE as u64 == 0, "misaligned frame");
        assert!(f2 % memory::physical::FRAME_SIZE as u64 == 0, "misaligned frame");
        a.free_frame(f1);
        a.free_frame(f2);
        assert!(a.free_frames() == before, "free count out of balance");
        println!("frame allocator self test passed ({:#x}, {:#x})", f1, f2);
    });
}

/// pull the first framebuffer out of the limine response and wrap it.
fn framebuffer_from_limine() -> Framebuffer {
    let resp = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("limine gave no framebuffer response");
    let fb = resp
        .framebuffers()
        .next()
        .expect("limine response holds no framebuffers");
    // limine guarantees the mapping is live and covers height * pitch.
    // nothing else draws, so we own it.
    unsafe {
        Framebuffer::new(
            fb.addr(),
            fb.width() as usize,
            fb.height() as usize,
            fb.pitch() as usize,
            fb.bpp() as usize,
        )
    }
}
