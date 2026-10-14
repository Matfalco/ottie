"""Writes the pairs chosen in the picker as Ottie's sound sets:
<out>/<set>-finished.ogg and <set>-failed.ogg (Ogg Vorbis), the pairs in
their order, named by <set-ids>. Checks that encoding kept their loudness.

Needs the `soundfile` package (libsndfile, with Vorbis), besides numpy.

Usage: python -I export.py <picker-dir> <out-dir> <set-ids>
  <set-ids>: comma-separated, one per pair, e.g. marimba,log,driftwood
"""
import json
import sys
from pathlib import Path

import soundfile

sys.path.insert(0, str(Path(__file__).parent))
from loudness import momentary  # noqa: E402

# Vorbis quality: 0 is the best, 1 the smallest.
COMPRESSION = 0.4


def main():
    picker, out, ids = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3].split(",")
    pairs = json.loads((picker / "choices.json").read_text(encoding="utf-8"))["pairs"]
    complete = [pair for pair in pairs if pair.get("success") and pair.get("failure")]
    if len(complete) != len(ids):
        sys.exit(f"{len(complete)} complete pairs, {len(ids)} set IDs")
    total = 0
    for set_id, pair in zip(ids, complete):
        for kind, key in (("finished", "success"), ("failed", "failure")):
            data, rate = soundfile.read(picker / "sounds" / f"{pair[key]}.wav", always_2d=True)
            path = out / f"{set_id}-{kind}.ogg"
            soundfile.write(path, data, rate, format="OGG", subtype="VORBIS", compression_level=COMPRESSION)
            back, _ = soundfile.read(path, always_2d=True)
            before = momentary(list(data.T), rate, 0.2)
            after = momentary(list(back.T), rate, 0.2)
            size = path.stat().st_size
            total += size
            print(f"{path.name:26} from {pair[key]:24} {size / 1024:5.1f} KB  {before:6.2f} -> {after:6.2f} LUFS")
    print(f"{total / 1024:.0f} KB in all")


if __name__ == "__main__":
    main()
