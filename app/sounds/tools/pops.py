"""Generates UI-style notification sounds (pops, bubbles, drops, dings,
plucks, blips), each set to the same loudness with a capped peak, plus a page
to listen to them side by side.

Usage: python -I pops.py <out-dir> <offset-db>
  offset-db: relative to Windows' notification sound (e.g. -3), for both its
  loudness and its peak.
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
rng = np.random.default_rng(7)


def times(duration):
    return np.arange(round(duration * RATE)) / RATE


def tone(freq, duration):
    """A sine whose frequency follows `freq` (a number or an array over time)."""
    t = times(duration)
    f = np.broadcast_to(np.asarray(freq, dtype=float), t.shape)
    return np.sin(2 * math.pi * np.cumsum(f) / RATE)


def envelope(duration, attack=0.002, decay=30.0):
    t = times(duration)
    return np.minimum(1, t / attack) * np.exp(-t * decay)


def glide(f0, f1, duration, speed=0.025):
    """From f0 towards f1, most of the way within `speed` seconds."""
    t = times(duration)
    return f1 + (f0 - f1) * np.exp(-t / speed)


def pop(f0, f1, duration=0.16, decay=28.0, speed=0.025):
    f = glide(f0, f1, duration, speed)
    body = tone(f, duration) + 0.15 * tone(2 * f, duration)
    return body * envelope(duration, 0.002, decay)


def bubble(f0, f1, duration=0.11):
    t = times(duration)
    f = f0 * (f1 / f0) ** (t / duration)
    # A bump, not a click: rises and falls within the bubble.
    return tone(f, duration) * np.sin(math.pi * t / duration) ** 1.5


def plink(f0, f1, duration=0.09):
    f = glide(f0, f1, duration, 0.012)
    return tone(f, duration) * envelope(duration, 0.001, 45)


def bell(freq, duration=0.9, brightness=1.0):
    partials = [(1, 1, 5), (2.76, 0.45 * brightness, 9), (5.40, 0.2 * brightness, 16), (8.93, 0.1 * brightness, 24)]
    out = np.zeros(round(duration * RATE))
    for ratio, gain, decay in partials:
        out += gain * tone(freq * ratio, duration) * envelope(duration, 0.002, decay)
    return out


def pluck(freq, duration=0.6):
    """Karplus-Strong: a burst of noise through a short delay that softens."""
    period = int(RATE / freq)
    buffer = rng.uniform(-1, 1, period)
    out = np.zeros(round(duration * RATE))
    for i in range(len(out)):
        j = i % period
        out[i] = buffer[j]
        buffer[j] = 0.996 * 0.5 * (buffer[j] + buffer[(j + 1) % period])
    return out * envelope(duration, 0.001, 3)


def blip(freq, duration=0.07):
    return np.tanh(2.2 * tone(freq, duration)) / np.tanh(2.2) * envelope(duration, 0.002, 40)


def tap(freq, duration=0.12):
    noise = rng.uniform(-1, 1, round(duration * RATE))
    # A one-pole low-pass: the knock of wood, not a hiss.
    soft = np.zeros_like(noise)
    for i in range(1, len(noise)):
        soft[i] = soft[i - 1] + 0.25 * (noise[i] - soft[i - 1])
    return (tone(freq, duration) + 0.5 * soft) * envelope(duration, 0.001, 55)


def mix(*parts):
    """(start s, samples) pieces, laid over each other."""
    length = max(round(start * RATE) + len(s) for start, s in parts) + round(0.03 * RATE)
    out = np.zeros(length)
    for start, s in parts:
        begin = round(start * RATE)
        out[begin : begin + len(s)] += s
    return out


# (group, id, name, sound, description)
CANDIDATES = [
    ("Pops and bubbles", "pop", "Pop", lambda: pop(380, 880), "One bubbly pop, pitch gliding up (the Messenger kind)"),
    ("Pops and bubbles", "pop-pop", "Pop pop", lambda: mix((0, pop(380, 880)), (0.11, pop(450, 1050))), "Two pops, the second higher"),
    ("Pops and bubbles", "pop-low", "Soft pop", lambda: pop(220, 520, 0.2, 22), "A lower, rounder pop"),
    ("Pops and bubbles", "bubble", "Bubble", lambda: bubble(260, 900), "A bubble rising to the surface"),
    ("Pops and bubbles", "bubbles", "Bubbles", lambda: mix((0, bubble(260, 700)), (0.09, bubble(330, 900)), (0.18, bubble(420, 1150))), "Three bubbles, each higher"),
    ("Drops", "droplet", "Droplet", lambda: plink(700, 2000), "A drop falling in water: a quick plink"),
    ("Drops", "droplets", "Droplets", lambda: mix((0, plink(650, 1800)), (0.12, plink(800, 2300))), "Two drops"),
    ("Drops", "drip-splash", "Drip", lambda: mix((0, bubble(200, 500, 0.09)), (0.07, plink(900, 2400))), "A low bubble, then a high plink"),
    ("Dings and bells", "ding", "Ding", lambda: bell(1318.5), "A small bell (E6)"),
    ("Dings and bells", "ding-dong", "Ding dong", lambda: mix((0, bell(1318.5, 0.7)), (0.16, bell(1046.5, 0.9))), "Two bell notes, E6 → C6"),
    ("Dings and bells", "glass", "Glass", lambda: bell(2093, 0.5, 0.6), "A tap on a glass (C7), short"),
    ("Dings and bells", "soft-chime", "Soft chime", lambda: bell(880, 1.0, 0.4), "A low, soft bell (A5)"),
    ("Plucks, blips and taps", "pluck-up", "Pluck", lambda: mix((0, pluck(440, 0.5)), (0.1, pluck(659.3))), "Two plucked strings, A4 → E5"),
    ("Plucks, blips and taps", "pluck", "Single pluck", lambda: pluck(587.3), "One plucked string (D5)"),
    ("Plucks, blips and taps", "blip-blip", "Blip blip", lambda: mix((0, blip(1000)), (0.09, blip(1500))), "Two soft electronic blips, up"),
    ("Plucks, blips and taps", "wood-tap", "Wood tap", lambda: mix((0, tap(600)), (0.1, tap(900))), "Two knocks on wood"),
    ("For failures", "pop-down", "Pop down", lambda: pop(900, 420, 0.18, 24), "A pop gliding down"),
    ("For failures", "uh-oh", "Uh-oh", lambda: mix((0, pop(700, 640, 0.15)), (0.15, pop(520, 470, 0.2, 20))), "Two pops, the second lower: “uh-oh”"),
    ("For failures", "blip-down", "Blip down", lambda: mix((0, blip(1200)), (0.1, blip(800, 0.1))), "Two blips, down"),
    ("For failures", "dong", "Dong", lambda: mix((0, bell(1046.5, 0.6)), (0.15, bell(880, 0.9))), "Two bell notes falling a minor third (C6 → A5)"),
]


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
    system, system_peak = measure(str(SYSTEM))
    # Mono plays on both speakers: as loud as stereo with 3 dB less per channel.
    target = system + offset - 3.01
    peak_cap = system_peak + offset
    shutil.copy(SYSTEM, out / "00-windows-system.wav")
    rows = []
    for index, (group, ident, name, make, description) in enumerate(CANDIDATES, 1):
        samples = make()
        # Fade the tail out so it never clicks.
        fade = min(round(0.02 * RATE), len(samples) // 4)
        samples[-fade:] *= np.linspace(1, 0, fade)
        level = momentary([samples], RATE)
        gain = 10 ** ((target - level) / 20)
        peak = float(np.max(np.abs(samples))) * gain
        # A short sound reads quieter over 400 ms than it sounds: its peak caps it.
        gain *= min(1, 10 ** (peak_cap / 20) / peak)
        samples *= gain
        file = f"{index:02d}-{ident}.wav"
        write(out / file, samples)
        print(f"{file:24} {momentary([samples], RATE) + 3.01:7.2f} LUFS  peak {20 * math.log10(np.max(np.abs(samples))):6.2f} dBFS")
        rows.append((group, file, name, description))
    page(out, rows, offset)


def page(out, rows, offset):
    groups = {}
    for group, file, name, description in rows:
        groups.setdefault(group, []).append((file, name, description))
    sections = []
    for group, items in groups.items():
        cards = "".join(
            f'<button data-src="{html.escape(file)}"><span class="play">▶</span>'
            f"<span><b>{html.escape(name)}</b><small>{html.escape(description)}</small>"
            f"<code>{html.escape(file)}</code></span></button>"
            for file, name, description in items
        )
        sections.append(f"<h2>{html.escape(group)}</h2><div class=grid>{cards}</div>")
    (out / "index.html").write_text(
        f"""<!doctype html>
<html lang=en><meta charset=utf-8><meta name=viewport content="width=device-width, initial-scale=1">
<title>Ottie pops and dings</title>
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
<h1>Ottie pops and dings</h1>
<p>Short UI sounds, each about {abs(offset):g} dB quieter than Windows’ own notification sound. Click to play; compare with the system sound first.</p>
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
