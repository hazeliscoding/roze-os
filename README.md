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

Early days. The repository is being bootstrapped, starting with a freestanding Rust kernel that boots through Limine under QEMU.

### Milestones

- [ ] Boot (Limine + QEMU)
- [ ] Serial console
- [ ] Framebuffer graphics
- [ ] Kernel console (framebuffer text)
- [ ] Interrupts (GDT/IDT/exceptions)
- [ ] Physical memory allocator
- [ ] Kernel heap
- [ ] Timer
- [ ] PS/2 keyboard
- [ ] C interop
- [ ] DoomGeneric integration
- [ ] DOOM rendering
- [ ] DOOM input
- [ ] WAD loading
- [ ] Playable DOOM 🎮
- [ ] Boot menu

## Prerequisites

- Rust nightly toolchain
- QEMU (`qemu-system-x86_64`)
- Clang or GCC (for the DoomGeneric C code, later milestones)
- xorriso (for building the boot ISO)

## Building and running

Build and run instructions land with the first bootable milestone. The intended workflow:

```bash
cargo xtask build
cargo xtask run
cargo xtask debug   # QEMU with GDB stub
```

## DOOM WAD

Commercial DOOM WAD files are copyrighted and never committed to this repository. When the WAD milestone lands, place a legally obtained `doom1.wad` (shareware) or [Freedoom](https://freedoom.github.io/) WAD in `assets/` as documented there.

## License

TBD
