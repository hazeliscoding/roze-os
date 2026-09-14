/* c interop smoke test. if rust can call this and get 42 back, the
   clang -> elf object -> static lib -> rust-lld pipeline works and
   doomgeneric has a road in. */

int roze_test(void) {
    return 42;
}
