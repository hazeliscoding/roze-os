//! doom lives here.
//!
//! the engine is vendored c (doom/doomgeneric), the platform bridge
//! is c (doom/platform), and this module is the rust side: kernel
//! exports the bridge calls into, plus the entry that hands control
//! to doomgeneric for good.

pub mod bindings;

unsafe extern "C" {
    fn doomgeneric_Create(argc: i32, argv: *mut *mut u8);
    fn doomgeneric_Tick();
}

/// hand the machine to doom. never returns, doom is the workload.
pub fn run(wad: &'static [u8]) -> ! {
    bindings::set_wad(wad);

    // engine wants classic argv. static strings, nul terminated.
    static ARG0: &[u8] = b"doom\0";
    static ARG1: &[u8] = b"-iwad\0";
    static ARG2: &[u8] = b"doom1.wad\0";
    let mut argv = [
        ARG0.as_ptr() as *mut u8,
        ARG1.as_ptr() as *mut u8,
        ARG2.as_ptr() as *mut u8,
        core::ptr::null_mut(),
    ];

    unsafe {
        doomgeneric_Create(3, argv.as_mut_ptr());
        loop {
            doomgeneric_Tick();
        }
    }
}
