"""Adds the two earlier rounds (synthesized marimba-like chimes, synthesized
pops and dings) to the picker, at the picker's loudness, without touching the
sounds already there or the choices.

Usage: python -I add_rounds.py <work-dir>
  <work-dir> holds candidates/ (chimes.py's output), pops/ (pops.py's) and
  picker/ (library.py's).
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import chimes  # noqa: E402
import library  # noqa: E402
import pops  # noqa: E402
from loudness import momentary  # noqa: E402

FAILING = {"deep-fall", "mid-low-fall", "mid-fall", "mid-minor-fall", "high-fall", "mix-high-low", "mix-low-minor"}


def main():
    root = Path(sys.argv[1])
    picker = root / "picker"
    listing = json.loads((picker / "sounds.json").read_text(encoding="utf-8"))
    known = {sound["id"] for sound in listing}
    system = library.read(library.SYSTEM)
    target = momentary(list(system), library.RATE, library.WINDOW) - 3
    rounds = [
        ("chime", "First round: synthesized chimes", root / "candidates",
         [(ident, name, "failure" if ident in FAILING else "success", desc) for _, ident, name, _, desc in chimes.CANDIDATES]),
        ("pop", "Second round: synthesized pops", root / "pops",
         [(ident, name, "failure" if group == "For failures" else "success", desc) for group, ident, name, _, desc in pops.CANDIDATES]),
    ]
    added = 0
    for prefix, source, folder, items in rounds:
        for index, (ident, name, kind, description) in enumerate(items, 1):
            sound_id = f"{prefix}-{ident}"
            if sound_id in known:
                continue
            samples = library.read(folder / f"{index:02d}-{ident}.wav")
            samples = library.level(library.trim(samples), target, library.PEAK_CAP)
            library.write(picker / "sounds" / f"{sound_id}.wav", samples)
            listing.append(dict(id=sound_id, name=name, kind=kind, description=description,
                                seconds=round(samples.shape[1] / library.RATE, 2), source=source,
                                licence="MPL-2.0 (ours)", url=""))
            added += 1
    (picker / "sounds.json").write_text(json.dumps(listing, indent=1, ensure_ascii=False), encoding="utf-8")
    print(added, "added,", len(listing), "in all")


if __name__ == "__main__":
    main()
