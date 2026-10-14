"""Builds the sound picker's sounds: recordings from CC0 packs, two-note
sounds made from real instrument recordings (VCSL, CC0), and a few
synthesized ones, all at one loudness. Writes <out>/sounds/*.wav and
<out>/sounds.json.

Usage: python -I library.py <sources-dir> <out-dir> <offset-db>
  <sources-dir>: what fetch.py downloaded.
"""
import json
import math
import re
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from loudness import momentary  # noqa: E402

RATE = 48000
# Loudness over the loudest 200 ms: fair to short sounds as well as long ones.
WINDOW = 0.2
# Only to keep attacks from being harsh; loudness sets the level.
PEAK_CAP = -6.0
SYSTEM = Path(r"C:\Windows\media\Windows Notify System Generic.wav")
rng = np.random.default_rng(11)
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def midi(note):
    name, octave = re.fullmatch(r"([A-G]#?)(-?\d)", note).groups()
    return NAMES.index(name) + 12 * (int(octave) + 1)


# --- Reading any WAV ------------------------------------------------------


def read(path):
    """(channels × samples float array at RATE)."""
    data = Path(path).read_bytes()
    pos, fmt, raw = 12, None, None
    while pos + 8 <= len(data):
        tag, size = data[pos : pos + 4], struct.unpack("<I", data[pos + 4 : pos + 8])[0]
        body = data[pos + 8 : pos + 8 + size]
        if tag == b"fmt ":
            fmt = struct.unpack("<HHIIHH", body[:16])
            if fmt[0] == 0xFFFE:  # extensible: the real format is in the subformat
                fmt = (struct.unpack("<H", body[24:26])[0],) + fmt[1:]
        elif tag == b"data":
            raw = body
        pos += 8 + size + (size & 1)
    kind, channels, rate, _, _, bits = fmt
    if kind == 3:
        samples = np.frombuffer(raw[: len(raw) // (bits // 8) * (bits // 8)], dtype="<f4" if bits == 32 else "<f8").astype(float)
    elif bits == 24:
        b = np.frombuffer(raw[: len(raw) // 3 * 3], dtype=np.uint8).reshape(-1, 3)
        ints = (b[:, 0].astype(np.int32) | (b[:, 1].astype(np.int32) << 8) | (b[:, 2].astype(np.int32) << 16))
        ints = np.where(ints & 0x800000, ints - 0x1000000, ints)
        samples = ints / 2**23
    elif bits == 16:
        samples = np.frombuffer(raw[: len(raw) // 2 * 2], dtype="<i2") / 2**15
    elif bits == 32:
        samples = np.frombuffer(raw[: len(raw) // 4 * 4], dtype="<i4") / 2**31
    elif bits == 8:
        samples = (np.frombuffer(raw, dtype=np.uint8).astype(float) - 128) / 128
    else:
        raise ValueError(f"{path}: {bits} bits")
    samples = samples[: len(samples) // channels * channels].reshape(-1, channels).T
    return resample(samples, rate / RATE)


def resample(samples, ratio):
    """Reads the samples `ratio` times faster (pitch × ratio)."""
    n = samples.shape[1]
    positions = np.arange(0, n - 1, ratio)
    return np.stack([np.interp(positions, np.arange(n), c) for c in samples])


def stereo(samples):
    return samples if samples.shape[0] == 2 else np.repeat(samples[:1], 2, axis=0)


# --- Shaping ---------------------------------------------------------------


def trim(samples, longest=1.6, fade=0.3):
    level = np.max(np.abs(samples), axis=0)
    peak = level.max()
    loud = np.nonzero(level > peak * 10 ** (-45 / 20))[0]
    start = max(0, loud[0] - round(0.003 * RATE))
    end = min(samples.shape[1], loud[-1] + 1)
    samples = samples[:, start:end].copy()
    if samples.shape[1] > longest * RATE:
        samples = samples[:, : round(longest * RATE)]
        n = round(fade * RATE)
        samples[:, -n:] *= np.linspace(1, 0, n) ** 2
    n = min(round(0.01 * RATE), samples.shape[1] // 4)
    samples[:, -n:] *= np.linspace(1, 0, n)
    return samples


def level(samples, target, peak_cap):
    loud = momentary(list(samples), RATE, WINDOW)
    if samples.shape[0] == 1:
        loud += 3.01
    gain = 10 ** ((target - loud) / 20)
    peak = float(np.max(np.abs(samples))) * gain
    return samples * gain * min(1, 10 ** (peak_cap / 20) / peak)


def mix(*parts):
    """(start s, gain, stereo samples) laid over each other."""
    length = max(round(start * RATE) + s.shape[1] for start, _, s in parts)
    out = np.zeros((2, length))
    for start, gain, s in parts:
        begin = round(start * RATE)
        out[:, begin : begin + s.shape[1]] += gain * stereo(s)
    return out


def reverb(samples, wet=0.2, length=0.7, decay=0.16):
    """A small room: decorrelated noise fading out, one per side."""
    samples = stereo(samples)
    t = np.arange(round(length * RATE)) / RATE
    out = []
    for c in samples:
        ir = rng.standard_normal(len(t)) * np.exp(-t / decay)
        ir[: round(0.004 * RATE)] = 0
        ir /= np.sqrt(np.sum(ir**2))
        size = len(c) + len(ir)
        wet_part = np.fft.irfft(np.fft.rfft(c, size) * np.fft.rfft(ir, size), size)
        dry = np.pad(c, (0, len(ir)))
        out.append(dry + wet * wet_part)
    return np.array(out)


def write(path, samples):
    data = (np.clip(samples.T, -1, 1) * 32767).round().astype("<i2")
    channels = samples.shape[0]
    head = b"RIFF" + struct.pack("<I", 36 + data.nbytes) + b"WAVEfmt " + struct.pack(
        "<IHHIIHH", 16, 1, channels, RATE, RATE * 2 * channels, 2 * channels, 16
    ) + b"data" + struct.pack("<I", data.nbytes)
    Path(path).write_bytes(head + data.tobytes())


# --- Instruments from VCSL -------------------------------------------------


class Instrument:
    """Recordings by note; any other note is the nearest one, retuned."""

    def __init__(self, folder, pattern, files=None):
        self.notes = {}
        for path in sorted(Path(folder).glob("*.wav")):
            found = re.search(pattern, path.name)
            if found and (files is None or files in path.name):
                self.notes[midi(found.group(1))] = path
        self.cache = {}

    def __call__(self, note):
        want = midi(note)
        near = min(self.notes, key=lambda m: abs(m - want))
        if near not in self.cache:
            self.cache[near] = trim(stereo(read(self.notes[near])), 2.5, 0.5)
        return resample(self.cache[near], 2 ** ((want - near) / 12))


# --- Synthesized, closer to real -------------------------------------------


def struck(freq, partials, duration, damping, detune=0.004):
    """A resonating object hit by a short burst of noise: each mode rings
    at its own rate, slightly detuned so it beats like a real one."""
    t = np.arange(round(duration * RATE)) / RATE
    hit = rng.standard_normal(round(0.004 * RATE)) * np.hanning(round(0.004 * RATE))
    out = np.zeros(len(t))
    for ratio, gain, decay in partials:
        for side in (-1, 1):
            f = freq * ratio * (1 + side * detune * rng.uniform(0.3, 1))
            mode = np.sin(2 * math.pi * f * t + rng.uniform(0, 6.3)) * np.exp(-t * decay * damping)
            out += 0.5 * gain * mode
    excitation = np.convolve(out, hit)[: len(t)]
    return excitation[None, :]


def wood(freq, duration=0.5):
    return struck(freq, [(1, 1, 18), (3.93, 0.35, 40), (9.0, 0.12, 70)], duration, 1.0, 0.002)


def glass(freq, duration=1.0):
    return struck(freq, [(1, 1, 5), (2.32, 0.4, 8), (4.25, 0.2, 13), (6.63, 0.1, 20)], duration, 1.0, 0.0015)


def drop(freq, duration=0.25):
    """A drop: a bubble's ring rising in pitch as it closes."""
    t = np.arange(round(duration * RATE)) / RATE
    f = freq * np.exp(np.minimum(t, 0.05) * 14)
    ring = np.sin(2 * math.pi * np.cumsum(f) / RATE) * np.exp(-t * 32) * np.minimum(1, t / 0.0015)
    splash = rng.standard_normal(len(t)) * np.exp(-t * 120) * 0.05
    return (ring + splash)[None, :]


# --- The list ----------------------------------------------------------------

PACKS = {
    "kenney": dict(source="Kenney, Interface Sounds", licence="CC0 1.0", url="https://kenney.nl/assets/interface-sounds"),
    "lamb": dict(source="Robin Lamb, UI Sound Effects", licence="CC0 1.0", url="https://opengameart.org/node/141201"),
    "haeldb": dict(source="HaelDB, ui sounds", licence="CC0 1.0", url="https://opengameart.org/content/ui-sounds-0"),
    "vcsl": dict(source="Made from VCSL recordings", licence="CC0 1.0", url="https://github.com/sgossner/VCSL"),
    "synth": dict(source="Synthesized for Ottie", licence="MPL-2.0 (ours)", url=""),
}


def build(sources):
    sounds = []  # (id, name, kind, pack, description, samples)

    kenney = sources / "kenney/x/kenney-interface-sounds-master/addons/kenney_interface_sounds"
    for stem, count, kind in [("confirmation", 4, "success"), ("glass", 6, "success"), ("drop", 4, "success"),
                              ("pluck", 2, "success"), ("question", 4, "success"), ("bong", 1, "success"),
                              ("error", 8, "failure")]:
        for i in range(1, count + 1):
            sounds.append((f"kenney-{stem}-{i}", f"{stem.capitalize()} {i}", kind, "kenney", f"{stem}_{i:03d}.wav",
                           read(kenney / f"{stem}_{i:03d}.wav")))

    lamb = sources / "lamb/x/ui_wav"
    for file, name, kind in [("chimes", "Chimes", "success"), ("Ding", "Ding", "success"), ("ding_deep", "Deep ding", "success"),
                             ("alarm", "Alarm", "failure"), ("alarm2", "Alarm 2", "failure"), ("dum", "Dum", "failure"),
                             ("negative_sound", "Negative", "failure"), ("negative_sound2", "Negative 2", "failure")]:
        sounds.append((f"lamb-{file.lower()}", name, kind, "lamb", f"{file}.wav", read(lamb / f"{file}.wav")))

    haeldb = sources / "haeldb/x/New folder"
    for i in range(10):
        sounds.append((f"haeldb-{i}", f"Sound {i}", "success", "haeldb", f"#{i}.wav", read(haeldb / f"#{i}.wav")))
    sounds.append(("haeldb-wrong", "Wrong", "failure", "haeldb", "Wrong Error.wav", read(haeldb / "Wrong Error.wav")))

    v = sources / "vcsl"
    kal = Instrument(v / "kalimba", r"Main_([A-G]#?\d)_", "MBira3")
    kal_k = Instrument(v / "kalimba", r"MainSpirit_([A-G]#?\d)_", "Mbira6")
    glock = Instrument(v / "glockenspiel", r"soft_([A-G]#?\d)_")
    marimba = Instrument(v / "marimba", r"_([A-G]#?\d)_soft")
    chimes = Instrument(v / "hand-chimes", r"sus_([A-G]#?\d)_")
    woodblock = {n: trim(stereo(read(p))) for n, p in [("mp", v / "woodblock/wood_click_mp.wav"), ("pp", v / "woodblock/wood_click_pp_rr1.wav")]}
    slit = {n: trim(stereo(read(v / f"slit-drum/LogDrum{n}_MedM_v2_rr1_Sum.wav"))) for n in ("Hi", "Lo")}
    cymbal = trim(stereo(read(v / "finger-cymbals/Fing_Cymb.wav")), 1.6, 0.5)

    def seq(instrument, notes, gap=0.13, gains=None):
        gains = gains or [1] * len(notes)
        return mix(*[(i * gap, g, instrument(n)) for i, (n, g) in enumerate(zip(notes, gains))])

    made = [
        ("kalimba-rise", "Kalimba, up", "success", "Kalimba G4 → D5", seq(kal, ["G4", "D5"])),
        ("kalimba-arpeggio", "Kalimba, three up", "success", "Kalimba C5 E5 G5", seq(kal, ["C5", "E5", "G5"], 0.11)),
        ("kalimba-single", "Kalimba, one note", "success", "Kalimba E5", kal("E5")),
        ("kalimba-kenya-rise", "Kalimba (Kenya), up", "success", "Another kalimba, F#4 → B4", seq(kal_k, ["F#4", "B4"])),
        ("glock-rise", "Glockenspiel, up", "success", "Glockenspiel C6 → G6", seq(glock, ["C6", "G6"], 0.12)),
        ("glock-single", "Glockenspiel, one note", "success", "Glockenspiel G5", glock("G5")),
        ("glock-arpeggio", "Glockenspiel, three up", "success", "Glockenspiel C6 E6 G6", seq(glock, ["C6", "E6", "G6"], 0.1)),
        ("marimba-rise", "Marimba, up", "success", "Real marimba G4 → C5", seq(marimba, ["G4", "C5"])),
        ("marimba-third", "Marimba, third", "success", "Real marimba C5 → E5", seq(marimba, ["C5", "E5"], 0.12)),
        ("chime-single", "Hand chime", "success", "Hand chime E5", chimes("E5")),
        ("chime-rise", "Hand chimes, up", "success", "Hand chimes C5 → E5", seq(chimes, ["C5", "E5"], 0.18)),
        ("finger-cymbal", "Finger cymbal", "success", "One finger cymbal, a bright ting", cymbal),
        ("wood-glock", "Wood + glockenspiel", "success", "Woodblock tap, then glockenspiel C6", mix((0, 0.8, woodblock["pp"]), (0.09, 1, glock("C6")))),
        ("kalimba-fall", "Kalimba, down", "failure", "Kalimba D5 → A4", seq(kal, ["D5", "A4"])),
        ("kalimba-low", "Kalimba, low down", "failure", "Kalimba F4 → D4", seq(kal, ["F4", "D4"], 0.15)),
        ("kalimba-kenya-fall", "Kalimba (Kenya), down", "failure", "Another kalimba, B4 → F#4", seq(kal_k, ["B4", "F#4"])),
        ("glock-fall", "Glockenspiel, down", "failure", "Glockenspiel G5 → C5", seq(glock, ["G5", "C5"], 0.13)),
        ("marimba-fall", "Marimba, down", "failure", "Real marimba C5 → G4", seq(marimba, ["C5", "G4"])),
        ("marimba-knock", "Marimba, low knock", "failure", "Real marimba C4 twice", seq(marimba, ["C4", "C4"], 0.15, [1, 0.8])),
        ("chime-fall", "Hand chimes, down", "failure", "Hand chimes D5 → A4", seq(chimes, ["D5", "A4"], 0.18)),
        ("chime-low", "Hand chime, low", "failure", "Hand chime F#4", chimes("F#4")),
        ("slit-drum", "Slit drum, down", "failure", "Wooden slit drum, high then low", mix((0, 1, slit["Hi"]), (0.13, 1, slit["Lo"]))),
        ("woodblock-knock", "Woodblock knock", "failure", "Woodblock, two knocks", mix((0, 1, woodblock["mp"]), (0.12, 0.8, woodblock["mp"]))),
    ]
    for ident, name, kind, description, samples in made:
        sounds.append((f"vcsl-{ident}", name, kind, "vcsl", description, samples))

    synth = [
        ("drop", "Water drop", "success", "A drop of water, in a small room", reverb(drop(900))),
        ("drops", "Two drops", "success", "Two drops, the second higher", reverb(mix((0, 1, drop(850)), (0.13, 0.9, drop(1150))))),
        ("wood-rise", "Wooden bars, up", "success", "Two struck wooden bars, up a fifth", reverb(mix((0, 1, wood(523)), (0.12, 1, wood(784))))),
        ("glass-rise", "Glasses, up", "success", "Two struck glasses, up a fourth", reverb(mix((0, 1, glass(1175)), (0.13, 0.9, glass(1568))))),
        ("wood-fall", "Wooden bars, down", "failure", "Two struck wooden bars, down a fifth", reverb(mix((0, 1, wood(587)), (0.13, 1, wood(392))))),
        ("glass-fall", "Glasses, down", "failure", "Two struck glasses, down a minor third", reverb(mix((0, 1, glass(1319)), (0.14, 1, glass(1109))))),
    ]
    for ident, name, kind, description, samples in synth:
        sounds.append((f"synth-{ident}", name, kind, "synth", description, samples))
    return sounds


def main():
    sources, out, offset = Path(sys.argv[1]), Path(sys.argv[2]), float(sys.argv[3])
    folder = out / "sounds"
    folder.mkdir(parents=True, exist_ok=True)
    system = read(SYSTEM)
    target = momentary(list(system), RATE, WINDOW) + offset
    peak_cap = PEAK_CAP
    write(folder / "windows-system.wav", system)
    listing = []
    for ident, name, kind, pack, description, samples in build(sources):
        samples = level(trim(samples), target, peak_cap)
        write(folder / f"{ident}.wav", samples)
        listing.append(dict(id=ident, name=name, kind=kind, description=description,
                            seconds=round(samples.shape[1] / RATE, 2), **PACKS[pack]))
    (out / "sounds.json").write_text(json.dumps(listing, indent=1, ensure_ascii=False), encoding="utf-8")
    print(len(listing), "sounds")


if __name__ == "__main__":
    main()
