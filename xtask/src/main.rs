//! build orchestration for rozeos.
//!
//! subcommands:
//!   cargo xtask build    compile the kernel for x86_64-unknown-none
//!   cargo xtask image    build kernel and pack a bootable disk image
//!   cargo xtask run      image + boot it in qemu
//!   cargo xtask debug    same but qemu waits for gdb on :1234
//!
//! the disk image is a plain mbr + fat32 layout written entirely from
//! rust, no xorriso or mtools needed. uefi firmware finds the limine
//! executable at the removable media path efi/boot/bootx64.efi.

use std::error::Error;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

//=====================================================================
// paths and constants
//=====================================================================

/// limine binary release branch. pinned so builds are reproducible.
const LIMINE_REPO: &str = "https://github.com/limine-bootloader/limine.git";
const LIMINE_BRANCH: &str = "v8.x-binary";

/// 128 MiB image, room for a debug kernel and a full wad. partition
/// starts at the customary lba 2048.
const IMG_SIZE: u64 = 128 * 1024 * 1024;
const PART_START_LBA: u64 = 2048;
const SECTOR: u64 = 512;

fn workspace_root() -> PathBuf {
    // xtask lives at <root>/xtask, manifest dir gets us home
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    // run defaults to release, doom in a debug build is a slideshow.
    // build/image/debug default to debug for fast iteration and
    // useful symbols. either can be overridden.
    let explicit_release = std::env::args().any(|a| a == "--release");
    let explicit_debug = std::env::args().any(|a| a == "--debug");
    let release = match cmd.as_str() {
        "run" => !explicit_debug,
        _ => explicit_release,
    };
    let result = match cmd.as_str() {
        "build" => build_kernel(release).map(|_| ()),
        "image" => image(release).map(|_| ()),
        "run" => run(release, false),
        "debug" => run(release, true),
        _ => {
            eprintln!("usage: cargo xtask <build|image|run|debug> [--release|--debug]");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("xtask: {e}");
        std::process::exit(1);
    }
}

//=====================================================================
// kernel build
//=====================================================================

/// compile the kernel crate for the freestanding target. returns the
/// path to the elf.
fn build_kernel(release: bool) -> Result<PathBuf> {
    let root = workspace_root();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .args(["build", "-p", "kernel", "--target", "x86_64-unknown-none"]);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err("kernel build failed".into());
    }
    let profile = if release { "release" } else { "debug" };
    Ok(root.join(format!("target/x86_64-unknown-none/{profile}/kernel")))
}

//=====================================================================
// limine binaries
//=====================================================================

/// fetch prebuilt limine binaries if we do not have them yet. shallow
/// clone of the binary branch, gitignored.
fn ensure_limine(root: &Path) -> Result<PathBuf> {
    let dir = root.join("limine");
    if dir.join("BOOTX64.EFI").exists() {
        return Ok(dir);
    }
    println!("xtask: fetching limine binaries ({LIMINE_BRANCH})");
    let status = Command::new("git")
        .current_dir(root)
        .args([
            "clone",
            "--branch",
            LIMINE_BRANCH,
            "--depth=1",
            LIMINE_REPO,
            "limine",
        ])
        .status()?;
    if !status.success() {
        return Err("limine clone failed".into());
    }
    Ok(dir)
}

//=====================================================================
// disk image
//=====================================================================

/// build the bootable disk image: mbr with one efi system partition,
/// fat32 inside, limine + kernel + config on top.
fn image(release: bool) -> Result<PathBuf> {
    let root = workspace_root();
    let kernel = build_kernel(release)?;
    let limine_dir = ensure_limine(&root)?;
    let img_path = root.join("target/rozeos.img");

    let mut f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&img_path)?;
    f.set_len(IMG_SIZE)?;

    write_mbr(&mut f)?;

    // format the partition span as fat32 and mount it
    let part_start = PART_START_LBA * SECTOR;
    let slice = fscommon::StreamSlice::new(f, part_start, IMG_SIZE)?;
    let mut disk = fscommon::BufStream::new(slice);
    fatfs::format_volume(
        &mut disk,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .volume_label(*b"ROZEOS     "),
    )?;
    let fs = fatfs::FileSystem::new(disk, fatfs::FsOptions::new())?;
    {
        let rootdir = fs.root_dir();

        // uefi removable media path, firmware runs this with no nvram entry
        let efi = rootdir.create_dir("EFI")?;
        let boot = efi.create_dir("BOOT")?;
        copy_into(&boot, "BOOTX64.EFI", &limine_dir.join("BOOTX64.EFI"))?;

        let bootdir = rootdir.create_dir("boot")?;
        copy_into(&bootdir, "kernel", &kernel)?;

        // wad goes in as a limine module when the developer supplied
        // one. see assets/README.md, wads are never committed. doom
        // needs an IWAD, mod PWADs get refused here instead of dying
        // as a missing lump panic at runtime.
        let mut wad = None;
        for name in ["doom1.wad", "freedoom1.wad"] {
            let p = root.join("assets").join(name);
            if !p.exists() {
                continue;
            }
            let mut magic = [0u8; 4];
            fs::File::open(&p)?.read_exact(&mut magic)?;
            match &magic {
                b"IWAD" => {
                    wad = Some(p);
                    break;
                }
                b"PWAD" => println!(
                    "xtask: {} is a PWAD (mod data, not the game), skipping. \
                     doom needs an IWAD: shareware doom1.wad (~4 MiB) or freedoom1.wad",
                    p.display()
                ),
                _ => println!("xtask: {} is not a wad file, skipping", p.display()),
            }
        }
        match wad {
            Some(p) => {
                copy_into(&bootdir, "doom1.wad", &p)?;
                println!("xtask: wad packed from {}", p.display());
            }
            None => println!("xtask: no usable iwad in assets/, doom will not start"),
        }

        copy_into(&rootdir, "limine.conf", &root.join("limine.conf"))?;
    }
    fs.unmount()?;

    println!("xtask: image at {}", img_path.display());
    Ok(img_path)
}

/// classic mbr: one bootable partition, type 0xef (efi system),
/// spanning lba 2048 to the end. chs fields maxed out, nobody has
/// looked at those since the clinton administration.
fn write_mbr(f: &mut fs::File) -> Result<()> {
    let mut mbr = [0u8; 512];
    let p = &mut mbr[446..462];
    p[0] = 0x80; // bootable
    p[1] = 0xff;
    p[2] = 0xff;
    p[3] = 0xff;
    p[4] = 0xef; // efi system partition
    p[5] = 0xff;
    p[6] = 0xff;
    p[7] = 0xff;
    p[8..12].copy_from_slice(&(PART_START_LBA as u32).to_le_bytes());
    let sectors = (IMG_SIZE / SECTOR - PART_START_LBA) as u32;
    p[12..16].copy_from_slice(&sectors.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xaa;
    f.seek(SeekFrom::Start(0))?;
    f.write_all(&mbr)?;
    Ok(())
}

/// read a host file and write it into the fat image.
fn copy_into<IO: fatfs::ReadWriteSeek>(
    dir: &fatfs::Dir<IO>,
    name: &str,
    src: &Path,
) -> Result<()> {
    let mut data = Vec::new();
    fs::File::open(src)?.read_to_end(&mut data)?;
    let mut file = dir.create_file(name)?;
    file.truncate()?;
    file.write_all(&data)?;
    Ok(())
}

//=====================================================================
// qemu
//=====================================================================

/// locate qemu. path first, then the stock windows install dir.
fn find_qemu() -> Result<PathBuf> {
    if let Ok(p) = which("qemu-system-x86_64") {
        return Ok(p);
    }
    let stock = PathBuf::from(r"C:\Program Files\qemu\qemu-system-x86_64.exe");
    if stock.exists() {
        return Ok(stock);
    }
    Err("qemu-system-x86_64 not found, install qemu".into())
}

fn which(name: &str) -> Result<PathBuf> {
    let paths = std::env::var_os("PATH").ok_or("no PATH")?;
    for dir in std::env::split_paths(&paths) {
        for ext in ["", ".exe"] {
            let cand = dir.join(format!("{name}{ext}"));
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    Err(format!("{name} not in PATH").into())
}

/// boot the image under qemu with uefi firmware. serial goes to stdio
/// so kernel logs land in the terminal.
fn run(release: bool, debug: bool) -> Result<()> {
    let root = workspace_root();
    let img = image(release)?;
    let qemu = find_qemu()?;

    // firmware ships next to the qemu binary. code is read only, vars
    // get a writable per-project copy so nvram scribbles stay local.
    let share = qemu.parent().unwrap().join("share");
    let code = share.join("edk2-x86_64-code.fd");
    if !code.exists() {
        return Err(format!("uefi firmware missing: {}", code.display()).into());
    }
    let vars_src = share.join("edk2-i386-vars.fd");
    let vars = root.join("target/ovmf-vars.fd");
    if !vars.exists() {
        fs::copy(&vars_src, &vars)?;
    }

    let mut cmd = Command::new(&qemu);
    cmd.args(["-M", "q35", "-m", "512M", "-serial", "stdio", "-no-reboot"]);
    // hardware acceleration when the host offers it, tcg as fallback.
    // doom under pure emulation runs like a slideshow.
    cmd.args(["-accel", "whpx,kernel-irqchip=off", "-accel", "tcg"]);
    cmd.arg("-drive");
    cmd.arg(format!(
        "if=pflash,format=raw,readonly=on,file={}",
        code.display()
    ));
    cmd.arg("-drive");
    cmd.arg(format!("if=pflash,format=raw,file={}", vars.display()));
    cmd.arg("-drive");
    cmd.arg(format!("format=raw,file={}", img.display()));
    if debug {
        // -s listens on :1234, -S freezes the cpu until gdb says go
        cmd.args(["-s", "-S"]);
        println!("xtask: qemu frozen, attach with: gdb -ex \"target remote :1234\"");
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err("qemu exited with failure".into());
    }
    Ok(())
}
