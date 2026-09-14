# RozeOS Roadmap

The project moves in phases. Each phase leaves the system bootable in QEMU. Nothing in a later phase starts until the current phase works end to end.

## v0.1 — DOOM 🎯

The founding goal: boot RozeOS in QEMU and play DOOM with no host OS underneath.

| # | Milestone | Summary |
|---|-----------|---------|
| 0 | Dev environment | Rust workspace, freestanding target, Limine, bootable image, QEMU |
| 1 | Hello RozeOS | Serial logging, startup banner, panic handler |
| 2 | Framebuffer | Pixel/rect drawing from Limine framebuffer |
| 3 | Kernel console | Bitmap font text on screen, print macros |
| 4 | Interrupts | GDT, IDT, exception handlers, PIC |
| 5 | Physical memory | Limine memory map, frame allocator |
| 6 | Kernel heap | Rust `alloc` support: Box, Vec, String |
| 7 | Timer | PIT ticks, `ticks_ms()`, `sleep_ms()` |
| 8 | Keyboard | PS/2 driver, scancode decode, event queue |
| 9 | C interop | Rust calls compiled C, linking pipeline |
| 10 | DoomGeneric | Vendor engine, platform bridge |
| 11 | DOOM rendering | Engine framebuffer scaled to screen |
| 12 | DOOM input | Keyboard events mapped to DOOM keys |
| 13 | WAD loading | WAD as Limine module, no filesystem |
| 14 | Playable DOOM | Full gameplay end to end |
| 15 | Boot menu | Minimal launcher: DOOM, sysinfo, graphics test |

## Beyond v0.1 🔭

Everything past DOOM is exploratory and unordered until v0.1 ships. Rough shape:

### v0.2 — Sound and polish
- PC speaker, then a real sound device (SB16 or AC'97 under QEMU) for DOOM music and effects
- Double-buffered rendering, vsync-ish frame pacing
- Boot menu graphics test and system info screens fleshed out
- SIGIL design language applied to all kernel UI

### v0.3 — Storage and filesystems
- Block device driver (AHCI or virtio-blk)
- Read-only FAT32, load WADs and configs from disk instead of embedding
- Save games that survive reboot

### v0.4 — Userspace foundations
- User/kernel privilege separation
- ELF loading, syscall layer
- Cooperative then preemptive scheduling
- DOOM promoted from kernel guest to first userspace citizen

### v0.5 — A real little OS
- Simple shell over the kernel console
- Mouse input
- SMP bring-up
- A second ported game or toy app to prove the ABI

### Someday / maybe
- Networking (virtio-net, small TCP/IP stack)
- USB (xHCI) and real hardware boot from a stick
- Port of Quake

## Principles

- DOOM first. Nothing jumps the queue.
- Simple beats clever. Synchronous, single-core, monolithic until v0.4 forces the issue.
- Every phase boots in QEMU before the next begins.
- No speculative abstractions for future phases.
