# RozeOS 🌹

A small educational x86_64 operating system written in Rust. The goal for v0.1 is one thing:

> Boot RozeOS in QEMU and play DOOM without Linux, Windows, or another host OS underneath it.

No POSIX, no userspace, no networking. Just a kernel, a framebuffer, and DOOM.

## Architecture

```text
┌───────────────────────────────┐
│             DOOM              │
│         DoomGeneric           │
├───────────────────────────────┤
│        Platform Bridge        │
│   graphics/input/time/memory  │
├───────────────────────────────┤
│          RozeOS Kernel        │
├───────────────────────────────┤
│            Limine             │
├───────────────────────────────┤
│             QEMU              │
└───────────────────────────────┘
```

The kernel is deliberately simple: synchronous, single-core, monolithic, framebuffer-based. DOOM runs directly in kernel address space. That is intentional.

## Status 🚧

The kernel boots through Limine under QEMU (UEFI) and prints its startup banner over serial.

### Milestones

- [x] Boot (Limine + QEMU)
- [x] Serial console
- [x] Framebuffer graphics
- [x] Kernel console (framebuffer text)
- [x] Interrupts (GDT/IDT/exceptions)
- [x] Physical memory allocator
- [x] Kernel heap
- [x] Timer
- [x] PS/2 keyboard
- [x] C interop
- [x] DoomGeneric integration
- [ ] DOOM rendering
- [ ] DOOM input
- [ ] WAD loading
- [ ] Playable DOOM 🎮
- [ ] Boot menu

## Prerequisites

- Rust (stable, via rustup; the `x86_64-unknown-none` target is pulled in by `rust-toolchain.toml`)
- QEMU (`qemu-system-x86_64`) with its bundled edk2 UEFI firmware
- Git (xtask fetches Limine binaries with it)
- Clang or GCC (for the DoomGeneric C code, later milestones)

## Building and running

```bash
cargo xtask build    # compile the kernel (x86_64-unknown-none)
cargo xtask image    # build target/rozeos.img (MBR + FAT32, written in pure Rust)
cargo xtask run      # boot the image in QEMU, serial on stdio
cargo xtask debug    # same, but QEMU waits for GDB on :1234
```

The boot image is UEFI-only for now: no xorriso or mtools needed, so it builds the same on Windows, Linux, and macOS. Kernel logs arrive over COM1.

## Debugging

`cargo xtask debug` starts QEMU frozen with a GDB stub:

```bash
gdb target/x86_64-unknown-none/debug/kernel -ex "target remote :1234"
```

## DOOM WAD

Commercial DOOM WAD files are copyrighted and never committed to this repository. When the WAD milestone lands, place a legally obtained `doom1.wad` (shareware) or [Freedoom](https://freedoom.github.io/) WAD in `assets/` as documented there.

## License

TBD
