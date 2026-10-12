# 0007: Unfinished downloads end in `.part.ottie`

Accepted 2026-10-12.

**Decision:** while downloading, a file is written to `<final name>.part.ottie` (for example `video.mp4.part.ottie`) and renamed to `<final name>` when complete. The suffix is the constant `PART_SUFFIX` in `crates/core/src/filename.rs`.

**Why**
- **Unambiguous:** Firefox also writes `.part` files, and the two could clash in a shared Downloads folder. When Ottie scans for its partial files on startup, it must never mistake another app's.
- **One extension for everything Ottie:** operating systems treat the last extension as the file type, so this is a `.ottie` file. A single file association can later cover partial files and any future Ottie files (exports, queues). Ottie decides what to do from the inner part.
- **Readable:** "part" is the established word for an incomplete download. With Windows' "hide known extensions" setting on, a registered `.ottie` shows as `video.mp4.part`, which doesn't look finished; a plain `.ottiedl` would show as `video.mp4`.

**Alternatives considered:** `.part` (clashes with Firefox), `.ottiedl`, `.ottie`, `.ottiepart`, `.dl.ottie`.

**Consequence:** if this changes after release, the startup scan has to recognise the old suffix too.
