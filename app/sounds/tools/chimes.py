"""Generates notification-sound candidates (marimba-like chimes), each set to
the same loudness, plus a page to listen to them side by side.

Usage: python -I chimes.py <out-dir> <offset-db>
  offset-db: loudness relative to Windows' notification sound (e.g. -3).
"""
import html
import math
import shutil
import sys
import wave
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from loudness import measure, momentary  # noqa: E402

RATE = 48000
SYSTEM = Path(r"C:\Windows\media\Windows Notify System Generic.wav")

NOTE = {}
for octave in range(2, 8):
    for i, name in enumerate(["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]):
        NOTE[f"{name}{octave}"] = 440 * 2 ** ((i - 9) / 12 + (octave - 4))

# (group, file id, name, notes: [(start s, note, gain)], description)
CANDIDATES = [
    ("Deep", "deep-rise", "Burrow", [(0, "C4", 1), (0.13, "G4", 1)], "C4 → G4, rising fifth"),
    ("Deep", "deep-fall", "Riverbed", [(0, "G4", 1), (0.14, "C4", 1)], "G4 → C4, falling fifth"),
    ("Deep", "deep-knock", "Knock", [(0, "A3", 1), (0.15, "A3", 0.8)], "A3 twice, a soft knock"),
    ("Deep", "deep-triad", "Driftwood", [(0, "C4", 1), (0.1, "E4", 0.9), (0.2, "G4", 0.9)], "C4 E4 G4, slow arpeggio"),
    ("Middle", "mid-rise", "Clapotis (current)", [(0, "E5", 1), (0.12, "B5", 1)], "E5 → B5, the current “finished”"),
    ("Middle", "mid-low-fall", "Galet (current)", [(0, "G4", 1), (0.14, "D4", 1)], "G4 → D4, the current “failed”"),
    ("Middle", "mid-fall", "Ebb", [(0, "B5", 1), (0.12, "E5", 1)], "B5 → E5, falling fifth"),
    ("Middle", "mid-triad", "Shell", [(0, "C5", 1), (0.09, "E5", 0.9), (0.18, "G5", 0.9)], "C5 E5 G5, quick arpeggio"),
    ("Middle", "mid-fourth", "Whisker", [(0, "A4", 1), (0.13, "D5", 1)], "A4 → D5, rising fourth"),
    ("Middle", "mid-minor-fall", "Kelp", [(0, "E5", 1), (0.1, "C5", 0.9), (0.2, "A4", 0.9)], "E5 C5 A4, falling minor, “oh no” but gentle"),
    ("High", "high-rise", "Spark", [(0, "C6", 1), (0.11, "G6", 0.9)], "C6 → G6, rising fifth"),
    ("High", "high-fall", "Droplet", [(0, "G6", 0.9), (0.11, "C6", 1)], "G6 → C6, falling fifth"),
    ("High", "high-sparkle", "Glint", [(0, "C6", 1), (0.07, "E6", 0.85), (0.14, "G6", 0.8), (0.21, "C7", 0.7)], "C6 E6 G6 C7, fast run up"),
    ("High", "high-double", "Tap tap", [(0, "A5", 1), (0.12, "A5", 0.8)], "A5 twice"),
    ("Mixed", "mix-low-high", "Leap", [(0, "C4", 1), (0.14, "C6", 0.85)], "C4 → C6, two octaves up"),
    ("Mixed", "mix-high-low", "Dive", [(0, "C6", 0.85), (0.14, "C4", 1)], "C6 → C4, two octaves down"),
    ("Mixed", "mix-chord-note", "Raft", [(0, "C4", 1), (0, "G5", 0.6), (0.16, "E5", 0.9)], "C4 + G5 together, then E5"),
    ("Mixed", "mix-bounce", "Skipping stone", [(0, "E5", 1), (0.1, "E4", 1), (0.2, "B4", 0.9)], "E5 → E4 → B4, a bounce"),
    ("Mixed", "mix-drop-chime", "Splash", [(0, "G4", 1), (0.15, "G5", 0.7), (0.15, "D6", 0.5)], "G4, then G5 + D6 together"),
    ("Mixed", "mix-low-minor", "Undertow", [(0, "A5", 0.8), (0.12, "A3", 1)], "A5 → A3, two octaves down, darker"),
]


def note(samples, start, freq, gain):
    begin = round(start * RATE)
    t = np.arange(len(samples) - begin) / RATE
    attack = np.minimum(1, t / 0.008)
    body = np.exp(-t * 7) * np.sin(2 * math.pi * freq * t)
    # Marimba's bright partial (~4x), fading much faster.
    bright = 0.25 * np.exp(-t * 30) * np.sin(2 * math.pi * freq * 3.98 * t)
    samples[begin:] += gain * attack * (body + bright)


def chime(notes):
    length = max(start for start, _, _ in notes) + 0.65
    samples = np.zeros(round(length * RATE))
    for start, name, gain in notes:
        note(samples, start, NOTE[name], gain)
    # Fade the tail out so it never clicks.
    fade = round(0.05 * RATE)
    samples[-fade:] *= np.linspace(1, 0, fade)
    return samples


def write(path, samples):
    data = (np.clip(samples, -1, 1) * 32767).round().astype("<i2")
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(data.tobytes())


def main():
    out = Path(sys.argv[1])
    offset = float(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    system, _ = measure(str(SYSTEM))
    # Mono plays on both speakers: as loud as stereo with 3 dB less per channel.
    target = system + offset - 3.01
    rows = []
    shutil.copy(SYSTEM, out / "00-windows-system.wav")
    for index, (group, ident, name, notes, description) in enumerate(CANDIDATES, 1):
        samples = chime(notes)
        level = momentary([samples], RATE)
        samples *= 10 ** ((target - level) / 20)
        file = f"{index:02d}-{ident}.wav"
        write(out / file, samples)
        peak = 20 * math.log10(float(np.max(np.abs(samples))))
        rows.append((group, file, name, description, peak))
        print(f"{file:28} peak {peak:6.2f} dBFS")
    page(out, rows, offset)


def page(out, rows, offset):
    groups = {}
    for group, file, name, description, _ in rows:
        groups.setdefault(group, []).append((file, name, description))
    sections = []
    for group, items in groups.items():
        cards = "".join(
            f'<button data-src="{html.escape(file)}"><span class="play">▶</span>'
            f"<span><b>{html.escape(name)}</b><small>{html.escape(description)}</small>"
            f"<code>{html.escape(file)}</code></span></button>"
            for file, name, description in items
        )
        sections.append(f"<h2>{group}</h2><div class=grid>{cards}</div>")
    (out / "index.html").write_text(
        f"""<!doctype html>
<html lang=en><meta charset=utf-8><meta name=viewport content="width=device-width, initial-scale=1">
<title>Ottie sound candidates</title>
<style>
:root {{ --bg:#f6f4ef; --card:#fff; --text:#1d2a2c; --soft:#5d6b6d; --line:#ddd6c8; --accent:#0f6e74; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#0f1c1e; --card:#16292c; --text:#e6efee; --soft:#9fb3b2; --line:#24403f; --accent:#f2b544; }} }}
body {{ margin:0; padding:24px 16px 48px; background:var(--bg); color:var(--text); font:15px/1.45 system-ui, sans-serif; }}
main {{ max-width:960px; margin:auto; }}
h1 {{ margin:0 0 4px; font-size:22px; }} h2 {{ margin:28px 0 10px; font-size:16px; color:var(--soft); }}
p {{ margin:0; color:var(--soft); }}
.grid {{ display:grid; grid-template-columns:repeat(auto-fill, minmax(260px, 1fr)); gap:10px; }}
button {{ display:flex; gap:12px; align-items:flex-start; text-align:left; padding:12px 14px; border:1px solid var(--line);
  border-radius:12px; background:var(--card); color:inherit; font:inherit; cursor:pointer; }}
button:hover, button.on {{ border-color:var(--accent); }}
.play {{ color:var(--accent); font-size:18px; line-height:1.2; }}
b, small, code {{ display:block; }} small {{ color:var(--soft); }} code {{ margin-top:4px; font-size:12px; color:var(--soft); }}
.ref {{ max-width:420px; margin-top:16px; }}
</style>
<main>
<h1>Ottie sound candidates</h1>
<p>Each one is set {abs(offset):g} dB quieter than Windows’ own notification sound (same timbre: a soft marimba-like chime). Click to play; compare with the system sound first.</p>
<div class=ref><button data-src="00-windows-system.wav"><span class="play">▶</span><span><b>Windows’ notification sound</b><small>For comparison</small><code>00-windows-system.wav</code></span></button></div>
{"".join(sections)}
</main>
<script>
const audio = new Audio();
for (const button of document.querySelectorAll('button')) {{
  button.addEventListener('click', () => {{
    document.querySelectorAll('button.on').forEach((b) => b.classList.remove('on'));
    button.classList.add('on');
    audio.src = button.dataset.src;
    audio.currentTime = 0;
    audio.play();
  }});
}}
</script>
""",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
