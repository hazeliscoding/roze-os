//! kernel services exported to the c side.
//!
//! everything the mini libc and the platform bridge call lands here.
//! names are the abi, changing one breaks the c build silently, so
//! do not.

use core::alloc::Layout;

use crate::graphics::console;
use crate::input::keyboard::{self, KeyCode};
use crate::print;
use crate::time::timer;

//=====================================================================
// wad registration
//=====================================================================

/// the one file our "filesystem" serves. set before doom starts.
static mut WAD: Option<&'static [u8]> = None;

/// register the wad blob the bootloader loaded. call once, before
/// doomgeneric_Create.
pub fn set_wad(wad: &'static [u8]) {
    unsafe {
        WAD = Some(wad);
    }
}

/// c: const unsigned char *roze_wad_data(long *len)
#[unsafe(no_mangle)]
extern "C" fn roze_wad_data(len: *mut i64) -> *const u8 {
    unsafe {
        match WAD {
            Some(w) => {
                *len = w.len() as i64;
                w.as_ptr()
            }
            None => {
                *len = 0;
                core::ptr::null()
            }
        }
    }
}

//=====================================================================
// heap for c. sizes ride in a header in front of the block so free
// and realloc can reconstruct the layout rust demands.
//=====================================================================

/// header size, keeps 16 byte alignment for whatever follows.
const HDR: usize = 16;

#[unsafe(no_mangle)]
extern "C" fn roze_malloc(size: usize) -> *mut u8 {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size + HDR, 16).unwrap();
    unsafe {
        let p = alloc::alloc::alloc(layout);
        if p.is_null() {
            return core::ptr::null_mut();
        }
        (p as *mut usize).write(size);
        p.add(HDR)
    }
}

#[unsafe(no_mangle)]
extern "C" fn roze_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = ptr.sub(HDR);
        let size = (base as *const usize).read();
        let layout = Layout::from_size_align(size + HDR, 16).unwrap();
        alloc::alloc::dealloc(base, layout);
    }
}

#[unsafe(no_mangle)]
extern "C" fn roze_realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    if ptr.is_null() {
        return roze_malloc(new_size);
    }
    if new_size == 0 {
        roze_free(ptr);
        return core::ptr::null_mut();
    }
    unsafe {
        let old_size = (ptr.sub(HDR) as *const usize).read();
        let p = roze_malloc(new_size);
        if p.is_null() {
            return core::ptr::null_mut();
        }
        core::ptr::copy_nonoverlapping(ptr, p, old_size.min(new_size));
        roze_free(ptr);
        p
    }
}

//=====================================================================
// logging, time, exit
//=====================================================================

/// c: void roze_log_write(const char *buf, long len)
#[unsafe(no_mangle)]
extern "C" fn roze_log_write(buf: *const u8, len: i64) {
    if buf.is_null() || len <= 0 {
        return;
    }
    let bytes = unsafe { core::slice::from_raw_parts(buf, len as usize) };
    // doom output is ascii, anything else renders as the box glyph
    for &b in bytes {
        print!("{}", b as char);
    }
}

#[unsafe(no_mangle)]
extern "C" fn roze_ticks_ms() -> u32 {
    timer::ticks_ms() as u32
}

#[unsafe(no_mangle)]
extern "C" fn roze_sleep_ms(ms: u32) {
    timer::sleep_ms(ms as u64);
}

/// c: void roze_exit(int status). doom exiting means the workload is
/// gone, treat it as a panic so the message stays on screen.
#[unsafe(no_mangle)]
extern "C" fn roze_exit(status: i32) -> ! {
    panic!("doom exited with status {status}");
}

//=====================================================================
// video
//=====================================================================

/// c: void roze_draw_frame(const uint32_t *frame, int w, int h)
///
/// doom renders xrgb into its own buffer, we blit it centered onto
/// the framebuffer. integer scaling arrives with the rendering
/// milestone, this copy proves the pipe.
#[unsafe(no_mangle)]
extern "C" fn roze_draw_frame(frame: *const u32, w: i32, h: i32) {
    if frame.is_null() {
        return;
    }
    let (w, h) = (w as usize, h as usize);
    let src = unsafe { core::slice::from_raw_parts(frame, w * h) };
    console::with(|c| {
        let fb = c.framebuffer();
        let ox = (fb.width().saturating_sub(w)) / 2;
        let oy = (fb.height().saturating_sub(h)) / 2;
        for y in 0..h {
            for x in 0..w {
                fb.put_pixel(ox + x, oy + y, src[y * w + x]);
            }
        }
    });
}

//=====================================================================
// input
//=====================================================================

// doomkeys.h values the engine expects back from DG_GetKey
const KEY_RIGHTARROW: u8 = 0xae;
const KEY_LEFTARROW: u8 = 0xac;
const KEY_UPARROW: u8 = 0xad;
const KEY_DOWNARROW: u8 = 0xaf;
const KEY_ESCAPE: u8 = 27;
const KEY_ENTER: u8 = 13;
const KEY_FIRE: u8 = 0xa3;
const KEY_USE: u8 = 0xa2;
const KEY_RSHIFT: u8 = 0x80 + 0x36;
const KEY_LALT: u8 = 0x80 + 0x38;

/// kernel key event to doom key code. zero means drop it.
fn doom_key(code: KeyCode) -> u8 {
    match code {
        KeyCode::Up => KEY_UPARROW,
        KeyCode::Down => KEY_DOWNARROW,
        KeyCode::Left => KEY_LEFTARROW,
        KeyCode::Right => KEY_RIGHTARROW,
        KeyCode::Escape => KEY_ESCAPE,
        KeyCode::Enter => KEY_ENTER,
        KeyCode::Space => KEY_USE,
        KeyCode::LCtrl => KEY_FIRE,
        KeyCode::LShift | KeyCode::RShift => KEY_RSHIFT,
        KeyCode::LAlt => KEY_LALT,
        // doom wants lowercase ascii for letters and digits
        KeyCode::Char(c) => c.to_ascii_lowercase(),
        KeyCode::Unknown(_) => 0,
    }
}

/// c: int roze_get_key(int *pressed, unsigned char *key)
///
/// returns 1 with an event filled in, 0 when the queue is dry.
#[unsafe(no_mangle)]
extern "C" fn roze_get_key(pressed: *mut i32, key: *mut u8) -> i32 {
    while let Some(ev) = keyboard::poll_event() {
        let k = doom_key(ev.code);
        if k == 0 {
            continue;
        }
        unsafe {
            *pressed = ev.pressed as i32;
            *key = k;
        }
        return 1;
    }
    0
}
