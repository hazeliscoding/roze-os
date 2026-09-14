// kernel build script.
//
// two jobs: hand the linker our script, and run the c side of the
// house through clang. c objects are compiled freestanding for the
// same bare metal target as the kernel, archived, and linked in
// statically. doomgeneric rides this same path later.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// c sources local to the kernel crate.
const C_SOURCES: &[&str] = &["cbits/roze_test.c"];

/// platform bridge and mini libc, relative to the workspace root.
const PLATFORM_SOURCES: &[&str] = &[
    "doom/platform/roze_libc.c",
    "doom/platform/doomgeneric_roze.c",
];

/// vendored doomgeneric objects, the makefile list minus the other
/// platforms' backends. relative to doom/doomgeneric.
const DOOM_SOURCES: &[&str] = &[
    "am_map.c", "d_event.c", "d_items.c", "d_iwad.c", "d_loop.c", "d_main.c",
    "d_mode.c", "d_net.c", "doomdef.c", "doomgeneric.c", "doomstat.c",
    "dstrings.c", "dummy.c", "f_finale.c", "f_wipe.c", "g_game.c", "gusconf.c",
    "hu_lib.c", "hu_stuff.c", "i_cdmus.c", "i_endoom.c", "i_input.c",
    "i_joystick.c", "i_scale.c", "i_sound.c", "i_system.c", "i_timer.c",
    "i_video.c", "info.c", "m_argv.c", "m_bbox.c", "m_cheat.c", "m_config.c",
    "m_controls.c", "m_fixed.c", "m_menu.c", "m_misc.c", "m_random.c",
    "memio.c", "mus2mid.c", "p_ceilng.c", "p_doors.c", "p_enemy.c",
    "p_floor.c", "p_inter.c", "p_lights.c", "p_map.c", "p_maputl.c",
    "p_mobj.c", "p_plats.c", "p_pspr.c", "p_saveg.c", "p_setup.c",
    "p_sight.c", "p_spec.c", "p_switch.c", "p_telept.c", "p_tick.c",
    "p_user.c", "r_bsp.c", "r_data.c", "r_draw.c", "r_main.c", "r_plane.c",
    "r_segs.c", "r_sky.c", "r_things.c", "s_sound.c", "sha1.c", "sounds.c",
    "st_lib.c", "st_stuff.c", "statdump.c", "tables.c", "v_video.c",
    "w_checksum.c", "w_file.c", "w_file_stdc.c", "w_main.c", "w_wad.c",
    "wi_stuff.c", "z_zone.c",
];

/// flags matching the x86_64-unknown-none target: freestanding, no
/// sse or red zone (interrupt handlers do not save fp state and can
/// land on the stack any time), kernel code model for the higher
/// half.
const C_FLAGS: &[&str] = &[
    "-target",
    "x86_64-unknown-none-elf",
    "-ffreestanding",
    "-fno-stack-protector",
    "-fno-builtin",
    "-mno-red-zone",
    "-mno-sse",
    "-mno-mmx",
    // clang 22 ignores -msoft-float on x86, feed the feature straight
    // to the backend. matches the rust target's +soft-float abi.
    "-Xclang",
    "-target-feature",
    "-Xclang",
    "+soft-float",
    "-mcmodel=kernel",
    "-O2",
    "-Wall",
    "-Wextra",
];

fn tool(name: &str) -> String {
    // path first, stock llvm install dir as fallback
    let stock = format!(r"C:\Program Files\LLVM\bin\{name}.exe");
    if PathBuf::from(&stock).exists() {
        stock
    } else {
        name.into()
    }
}

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{dir}/linker.ld");
    println!("cargo:rerun-if-changed=linker.ld");

    let root = PathBuf::from(&dir).parent().unwrap().to_path_buf();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let clang = tool("clang");
    let ar = tool("llvm-ar");
    let libc_inc = format!("-I{}", root.join("doom/platform/include").display());
    let dg_inc = format!("-I{}", root.join("doom/doomgeneric").display());

    let mut objects = Vec::new();
    let mut compile = |src: &PathBuf, extra: &[&str]| {
        println!("cargo:rerun-if-changed={}", src.display());
        let stem = src.file_stem().unwrap().to_string_lossy().into_owned();
        let obj = out.join(format!("{stem}.o"));
        let status = Command::new(&clang)
            .args(C_FLAGS)
            .args(extra)
            .arg("-c")
            .arg(src)
            .arg("-o")
            .arg(&obj)
            .status()
            .expect("failed to run clang");
        assert!(status.success(), "clang failed on {}", src.display());
        objects.push(obj);
    };

    for src in C_SOURCES {
        compile(&PathBuf::from(&dir).join(src), &["-Wall", "-Wextra"]);
    }
    for src in PLATFORM_SOURCES {
        compile(&root.join(src), &["-Wall", "-Wextra", &libc_inc, &dg_inc]);
    }
    // vendored code compiles quiet (-w), its warnings are upstream's,
    // and we are told not to significantly modify the engine
    for src in DOOM_SOURCES {
        compile(
            &root.join("doom/doomgeneric").join(src),
            &["-w", "-std=gnu99", &libc_inc],
        );
    }

    let lib = out.join("libroze_c.a");
    let status = Command::new(&ar)
        .arg("crs")
        .arg(&lib)
        .args(&objects)
        .status()
        .expect("failed to run llvm-ar");
    assert!(status.success(), "llvm-ar failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=roze_c");
    println!("cargo:rerun-if-changed={}", root.join("doom/platform/include").display());
}
