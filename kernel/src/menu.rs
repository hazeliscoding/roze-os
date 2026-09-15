//! boot menu.
//!
//! minimal launcher drawn straight on the framebuffer. visual spec is
//! the sigil design system, theme 01 comp: obsidian surfaces, phosphor
//! text, terminal cyan for selection, square corners on a 4 px grid.
//! doom stays the main act, this is just the lobby.

use alloc::format;
use alloc::string::String;

use crate::arch::x86_64::power;
use crate::graphics::console;
use crate::graphics::font;
use crate::graphics::framebuffer::{Color, Framebuffer};
use crate::input::keyboard::{self, KeyCode, KeyEvent};
use crate::memory;
use crate::serial_println;
use crate::time::timer;

//=====================================================================
// sigil color tokens, theme 01 comp. source: tokens/colors.css
//=====================================================================

const SURFACE_BASE: Color = 0x0008_0b0d;
const SURFACE_RAISED: Color = 0x000b_1011;
const SURFACE_PANEL: Color = 0x0015_1b1c;
/// sigil selected surface is 10% cyan over the panel, precomputed
/// because the framebuffer has no alpha.
const SURFACE_SELECTED: Color = 0x001c_2f31;
const TEXT_PRIMARY: Color = 0x00e6_eee9;
const TEXT_SECONDARY: Color = 0x009b_aaa5;
const TEXT_MUTED: Color = 0x0065_736f;
/// terminal cyan, the primary interaction signal.
const SIGNAL_PRIMARY: Color = 0x005a_e4ed;
/// comp green.
const SIGNAL_SECONDARY: Color = 0x0044_d7a2;
const STATUS_WARNING: Color = 0x00e4_a84b;
const STATUS_DANGER: Color = 0x00d9_4d5b;
const BORDER_DEFAULT: Color = 0x0024_302f;
const BORDER_STRONG: Color = 0x003a_4a48;

//=====================================================================
// text on the raw framebuffer
//=====================================================================

/// draw one string at pixel (x, y), glyphs scaled up by an integer
/// factor, background painted so redraws do not smear.
fn draw_text(fb: &mut Framebuffer, x: usize, y: usize, scale: usize, fg: Color, bg: Color, s: &str) {
    for (i, b) in s.bytes().enumerate() {
        let glyph = &font::FONT[(b & 0x7f) as usize];
        let gx0 = x + i * font::GLYPH_W * scale;
        for (gy, bits) in glyph.iter().enumerate() {
            for gx in 0..font::GLYPH_W {
                let color = if bits & (1 << gx) != 0 { fg } else { bg };
                fb.fill_rect(gx0 + gx * scale, y + gy * scale, scale, scale, color);
            }
        }
    }
}

/// pixel width of a string at a given scale.
fn text_w(s: &str, scale: usize) -> usize {
    s.len() * font::GLYPH_W * scale
}

/// 1 px outline. sigil frames are inset borders, not shadows.
fn frame(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, color: Color) {
    fb.fill_rect(x, y, w, 1, color);
    fb.fill_rect(x, y + h - 1, w, 1, color);
    fb.fill_rect(x, y, 1, h, color);
    fb.fill_rect(x + w - 1, y, 1, h, color);
}

//=====================================================================
// input
//=====================================================================

/// block until a key event arrives, napping between ticks.
fn wait_key() -> KeyEvent {
    loop {
        if let Some(ev) = keyboard::poll_event() {
            return ev;
        }
        x86_64::instructions::hlt();
    }
}

/// block until escape goes down. the way back from every subscreen.
fn wait_for_escape() {
    loop {
        let ev = wait_key();
        if ev.pressed && ev.code == KeyCode::Escape {
            return;
        }
    }
}

//=====================================================================
// menu proper
//=====================================================================

const ENTRIES: [&str; 5] = [
    "Play DOOM",
    "System Info",
    "Graphics Test",
    "Reboot",
    "Shutdown",
];

/// panel geometry, all on the sigil 4 px grid.
const PANEL_W: usize = 480;
const PANEL_H: usize = 352;
const ROW_H: usize = 40;

/// run the launcher. never returns: doom takes the machine, reboot
/// and shutdown end it, everything else loops back here.
pub fn run(wad: Option<&'static [u8]>) -> ! {
    serial_println!("menu: up, {} entries", ENTRIES.len());
    let mut sel = 0usize;
    draw_menu(sel, wad);
    loop {
        let ev = wait_key();
        if !ev.pressed {
            continue;
        }
        match ev.code {
            KeyCode::Up => {
                sel = (sel + ENTRIES.len() - 1) % ENTRIES.len();
                draw_menu(sel, wad);
            }
            KeyCode::Down => {
                sel = (sel + 1) % ENTRIES.len();
                draw_menu(sel, wad);
            }
            KeyCode::Char(c @ b'1'..=b'5') => {
                sel = (c - b'1') as usize;
                draw_menu(sel, wad);
                activate(sel, wad);
                draw_menu(sel, wad);
            }
            KeyCode::Enter | KeyCode::Space => {
                activate(sel, wad);
                draw_menu(sel, wad);
            }
            _ => {}
        }
    }
}

/// fire the selected entry. subscreens return, the rest do not.
fn activate(sel: usize, wad: Option<&'static [u8]>) {
    match sel {
        0 => {
            if let Some(w) = wad {
                serial_println!("menu: starting doom");
                // clear so doom's startup log lands on a fresh screen
                console::with(|c| c.clear());
                crate::doom::run(w);
            }
            // no wad, entry is parked, ignore the press
        }
        1 => sysinfo_screen(wad),
        2 => graphics_test_screen(),
        3 => {
            serial_println!("menu: reboot");
            power::reboot();
        }
        4 => {
            serial_println!("menu: shutdown");
            power::shutdown();
        }
        _ => unreachable!(),
    }
}

/// paint the whole menu screen. redrawn in full on every input, at
/// five rows nobody can tell and nobody has to track damage.
fn draw_menu(sel: usize, wad: Option<&'static [u8]>) {
    console::with(|c| {
        let fb = c.framebuffer();
        fb.clear(SURFACE_BASE);

        let px = (fb.width().saturating_sub(PANEL_W)) / 2;
        let py = (fb.height().saturating_sub(PANEL_H)) / 2;
        fb.fill_rect(px, py, PANEL_W, PANEL_H, SURFACE_PANEL);
        frame(fb, px, py, PANEL_W, PANEL_H, BORDER_STRONG);

        // header: wordmark, version, divider
        let title = "RozeOS";
        let tx = px + (PANEL_W - text_w(title, 4)) / 2;
        draw_text(fb, tx, py + 24, 4, TEXT_PRIMARY, SURFACE_PANEL, title);
        let ver = "version 0.1";
        let vx = px + (PANEL_W - text_w(ver, 2)) / 2;
        draw_text(fb, vx, py + 64, 2, TEXT_MUTED, SURFACE_PANEL, ver);
        fb.fill_rect(px + 1, py + 92, PANEL_W - 2, 1, BORDER_DEFAULT);

        // entry rows
        let rows_y = py + 104;
        for (i, label) in ENTRIES.iter().enumerate() {
            draw_row(fb, px, rows_y + i * ROW_H, i, label, i == sel, wad);
        }

        // footer: divider and key hints
        let fy = rows_y + ENTRIES.len() * ROW_H + 8;
        fb.fill_rect(px + 1, fy, PANEL_W - 2, 1, BORDER_DEFAULT);
        let hint = "UP/DOWN SELECT   ENTER LAUNCH   1-5 JUMP";
        let hx = px + (PANEL_W - text_w(hint, 1)) / 2;
        draw_text(fb, hx, fy + 13, 1, TEXT_MUTED, SURFACE_PANEL, hint);

        draw_status_strip(fb, wad);
    });
}

/// one menu row. selection is the sigil recipe: tinted surface, cyan
/// edge bar, cyan index. a missing wad parks the doom row muted.
fn draw_row(
    fb: &mut Framebuffer,
    px: usize,
    y: usize,
    i: usize,
    label: &str,
    selected: bool,
    wad: Option<&'static [u8]>,
) {
    let parked = i == 0 && wad.is_none();
    let bg = if selected { SURFACE_SELECTED } else { SURFACE_PANEL };
    fb.fill_rect(px + 1, y, PANEL_W - 2, ROW_H, bg);
    if selected {
        fb.fill_rect(px + 1, y, 2, ROW_H, SIGNAL_PRIMARY);
    }

    let num_fg = if selected { SIGNAL_PRIMARY } else { TEXT_MUTED };
    let label_fg = if parked {
        TEXT_MUTED
    } else if selected {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let num = [b'[', b'1' + i as u8, b']'];
    let num = core::str::from_utf8(&num).unwrap();
    draw_text(fb, px + 24, y + 12, 2, num_fg, bg, num);
    draw_text(fb, px + 88, y + 12, 2, label_fg, bg, label);
    if parked {
        let x = px + 88 + text_w(label, 2) + 16;
        draw_text(fb, x, y + 16, 1, STATUS_WARNING, bg, "NO WAD");
    }
}

/// bottom status strip, sigil statusstrip flavor: raised surface, top
/// border, small mono facts. drawn on input, not on a clock.
fn draw_status_strip(fb: &mut Framebuffer, wad: Option<&'static [u8]>) {
    let h = 28;
    let y = fb.height() - h;
    let w = fb.width();
    fb.fill_rect(0, y, w, h, SURFACE_RAISED);
    fb.fill_rect(0, y, w, 1, BORDER_DEFAULT);

    let free_mib =
        memory::physical::with(|a| a.free_frames() * memory::physical::FRAME_SIZE / (1024 * 1024));
    let up = timer::ticks_ms() / 1000;
    let left = format!("MEM {} MIB FREE   UPTIME {}S", free_mib, up);
    draw_text(fb, 16, y + 10, 1, TEXT_SECONDARY, SURFACE_RAISED, &left);

    let (wtext, wcolor) = match wad {
        Some(b) => (format!("WAD {} KIB", b.len() / 1024), SIGNAL_SECONDARY),
        None => (String::from("WAD MISSING"), STATUS_WARNING),
    };
    let wx = w - 16 - text_w(&wtext, 1);
    draw_text(fb, wx, y + 10, 1, wcolor, SURFACE_RAISED, &wtext);
}

//=====================================================================
// subscreens
//=====================================================================

/// system info: the boot banner facts, sigil keyvalue style.
fn sysinfo_screen(wad: Option<&'static [u8]>) {
    console::with(|c| {
        let fb = c.framebuffer();
        fb.clear(SURFACE_BASE);

        let pw = 512;
        let ph = 384;
        let px = (fb.width().saturating_sub(pw)) / 2;
        let py = (fb.height().saturating_sub(ph)) / 2;
        fb.fill_rect(px, py, pw, ph, SURFACE_PANEL);
        frame(fb, px, py, pw, ph, BORDER_STRONG);

        draw_text(fb, px + 24, py + 24, 3, TEXT_PRIMARY, SURFACE_PANEL, "System Info");
        fb.fill_rect(px + 1, py + 60, pw - 2, 1, BORDER_DEFAULT);

        let free_mib = memory::physical::with(|a| {
            a.free_frames() * memory::physical::FRAME_SIZE / (1024 * 1024)
        });
        let (fw, fh, fbpp) = (fb.width(), fb.height(), fb.bpp());
        let rows: [(&str, String); 8] = [
            ("boot protocol", String::from("limine")),
            ("architecture", String::from("x86_64")),
            ("framebuffer", format!("{}x{}x{}", fw, fh, fbpp)),
            ("memory free", format!("{} MiB", free_mib)),
            ("kernel heap", format!("{} MiB", memory::heap::HEAP_SIZE / (1024 * 1024))),
            ("timer", format!("PIT {} Hz", timer::HZ)),
            ("uptime", format!("{} ms", timer::ticks_ms())),
            (
                "wad",
                match wad {
                    Some(b) => format!("{} KiB", b.len() / 1024),
                    None => String::from("not loaded"),
                },
            ),
        ];
        for (i, (k, v)) in rows.iter().enumerate() {
            let y = py + 80 + i * 32;
            draw_text(fb, px + 24, y, 2, TEXT_MUTED, SURFACE_PANEL, k);
            draw_text(fb, px + 240, y, 2, TEXT_PRIMARY, SURFACE_PANEL, v);
        }

        draw_text(fb, px + 24, py + ph - 24, 1, TEXT_MUTED, SURFACE_PANEL, "ESC BACK");
    });
    wait_for_escape();
}

/// graphics test: palette swatches, channel ramps, font specimen.
/// enough to spot a broken blit or a swapped channel at a glance.
fn graphics_test_screen() {
    console::with(|c| {
        let fb = c.framebuffer();
        fb.clear(SURFACE_BASE);
        let w = fb.width();

        draw_text(fb, 32, 32, 3, TEXT_PRIMARY, SURFACE_BASE, "Graphics Test");

        // sigil raw palette swatches
        let palette: [Color; 12] = [
            TEXT_PRIMARY,
            TEXT_SECONDARY,
            TEXT_MUTED,
            SIGNAL_PRIMARY,
            SIGNAL_SECONDARY,
            0x0016_8cff, // electric blue
            0x0037_6bd9, // cold blue
            STATUS_WARNING,
            STATUS_DANGER,
            0x00cc_5edb, // magenta
            0x0063_dc8b, // success
            0x0010_2526, // deep petrol
        ];
        let sw = 64;
        for (i, &color) in palette.iter().enumerate() {
            let x = 32 + i * (sw + 8);
            fb.fill_rect(x, 96, sw, sw, color);
            frame(fb, x, 96, sw, sw, BORDER_STRONG);
        }

        // channel ramps, one bar per primary. a swapped channel shows
        // up here as the wrong bar lighting the wrong color.
        let bar_w = w - 64;
        for (bi, shift) in [16u32, 8, 0].iter().enumerate() {
            let y = 192 + bi * 48;
            for i in 0..bar_w {
                let v = (i * 255 / bar_w.max(1)) as u32;
                fb.fill_rect(32 + i, y, 1, 40, v << shift);
            }
            frame(fb, 32, y, bar_w, 40, BORDER_DEFAULT);
        }

        // font specimen at the scales the ui actually uses
        let specimen = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789";
        draw_text(fb, 32, 352, 1, TEXT_SECONDARY, SURFACE_BASE, specimen);
        draw_text(fb, 32, 372, 2, TEXT_SECONDARY, SURFACE_BASE, specimen);

        draw_text(fb, 32, fb.height() - 48, 1, TEXT_MUTED, SURFACE_BASE, "ESC BACK");
    });
    wait_for_escape();
}
