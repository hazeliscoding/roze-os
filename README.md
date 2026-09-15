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

The kernel boots through Limine under QEMU (UEFI) into a boot menu, and DOOM is one keypress away. All v0.1 milestones are done.

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
- [x] DOOM rendering
- [x] DOOM input
- [x] WAD loading
- [x] Playable DOOM 🎮
- [x] Boot menu

## Prerequisites

- Rust (stable, via rustup; the `x86_64-unknown-none` target is pulled in by `rust-toolchain.toml`)
- QEMU (`qemu-system-x86_64`) with its bundled edk2 UEFI firmware
- Git (xtask fetches Limine binaries with it)
- Clang or GCC (for the DoomGeneric C code, later milestones)

## Building and running

```bash
cargo xtask build    # compile the kernel (x86_64-unknown-none)
cargo xtask image    # build target/rozeos.img (MBR + FAT32, written in pure Rust)
cargo xtask run      # boot the image in QEMU, serial on stdio (release build)
cargo xtask debug    # same, but QEMU waits for GDB on :1234 (debug build)
cargo xtask test     # headless boot test, asserts on serial markers
```

`run` and `test` build release by default because DOOM in a debug build drops frames; pass `--debug` to override. The other commands default to debug, pass `--release` to override.

## Testing

`cargo xtask test` boots the real image in headless QEMU and fails on any panic or timeout. It watches serial for the boot self tests, then drives the menu by injecting scancodes through the QEMU monitor: opens System Info and the graphics test, and with a WAD packed launches DOOM and waits for the engine to finish init. No WAD, it exercises the shutdown path instead. Every kernel subsystem sits somewhere on that path, so a green run means boot, interrupts, memory, input, and the C bridge all still work.

The boot image is UEFI-only for now: no xorriso or mtools needed, so it builds the same on Windows, Linux, and macOS. Kernel logs arrive over COM1.

## Debugging

`cargo xtask debug` starts QEMU frozen with a GDB stub:

```bash
gdb target/x86_64-unknown-none/debug/kernel -ex "target remote :1234"
```

## DOOM WAD

Place a legally obtained `doom1.wad` (shareware) or [Freedoom](https://freedoom.github.io/) `freedoom1.wad` in `assets/`. The build packs it into the boot image and Limine loads it into RAM as a boot module, so DOOM needs no filesystem and no host OS to find its data. Without a WAD the kernel still boots to the menu, with the DOOM entry disabled.

Commercial WAD files are copyrighted and never committed to this repository (`assets/*.wad` is gitignored). See `assets/README.md`.

## Boot menu

The kernel lands in a small launcher styled after the SIGIL design system: Play DOOM, System Info, Graphics Test, Reboot, Shutdown. Arrows or digits to select, Enter to launch, Escape to leave a subscreen. Without a WAD the DOOM entry sits parked and everything else still works.

## License

TBD
