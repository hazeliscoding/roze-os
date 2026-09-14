// kernel build script.
//
// two jobs: hand the linker our script, and run the c side of the
// house through clang. c objects are compiled freestanding for the
// same bare metal target as the kernel, archived, and linked in
// statically. doomgeneric rides this same path later.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// c sources linked into the kernel.
const C_SOURCES: &[&str] = &["cbits/roze_test.c"];

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
    "-msoft-float",
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

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let clang = tool("clang");
    let ar = tool("llvm-ar");

    let mut objects = Vec::new();
    for src in C_SOURCES {
        println!("cargo:rerun-if-changed={src}");
        let stem = PathBuf::from(src).file_stem().unwrap().to_owned();
        let obj = out.join(format!("{}.o", stem.to_string_lossy()));
        let status = Command::new(&clang)
            .args(C_FLAGS)
            .arg("-c")
            .arg(format!("{dir}/{src}"))
            .arg("-o")
            .arg(&obj)
            .status()
            .expect("failed to run clang");
        assert!(status.success(), "clang failed on {src}");
        objects.push(obj);
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
}
