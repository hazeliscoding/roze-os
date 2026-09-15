# Game data

Drop a DOOM IWAD here. The build packs the first one it finds into the
boot image as `doom1.wad`:

```text
assets/
├── doom1.wad       # DOOM shareware (preferred name)
└── freedoom1.wad   # Freedoom Phase 1, free alternative
```

Sources:

- DOOM shareware `doom1.wad`: freely distributable, widely mirrored.
  The real one is an IWAD of roughly 4 MiB. Plenty of sites serve mod
  PWADs under the same name; the build checks the magic bytes and
  refuses those with a message instead of crashing in game.
- Freedoom: https://freedoom.github.io/ (grab `freedoom1.wad` from the
  release zip)

WAD files are **never committed** (`assets/*.wad` is gitignored).
Commercial WADs (`doom.wad`, `doom2.wad`) are copyrighted; if you own
one it works, but it stays on your disk.
