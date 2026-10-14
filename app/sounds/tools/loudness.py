"""Loudness of short sounds, as ITU-R BS.1770 measures it: K-weighting
(a high shelf for the ear's sensitivity, a high-pass), then the loudest
400 ms window ("momentary loudness"), in LUFS. Also the peak, in dBFS.

Usage: python -I loudness.py <file.wav>...
"""
import math
import sys
import wave

import numpy as np


def read(path):
    with wave.open(path) as w:
        rate, channels, width = w.getframerate(), w.getnchannels(), w.getsampwidth()
        data = np.frombuffer(w.readframes(w.getnframes()), dtype={2: "<i2", 4: "<i4"}[width])
    samples = data.astype(np.float64) / (2 ** (8 * width - 1))
    return samples.reshape(-1, channels).T, rate


def biquad(x, b, a):
    b0, b1, b2 = (v / a[0] for v in b)
    a1, a2 = a[1] / a[0], a[2] / a[0]
    y = np.zeros_like(x)
    x1 = x2 = y1 = y2 = 0.0
    for i, v in enumerate(x):
        out = b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
        x2, x1, y2, y1 = x1, v, y1, out
        y[i] = out
    return y


def k_weight(x, rate):
    # High shelf: +4 dB above ~1.5 kHz.
    gain, fc, q = 4.0, 1500.0, 1 / math.sqrt(2)
    big = 10 ** (gain / 40)
    w0 = 2 * math.pi * fc / rate
    alpha = math.sin(w0) / (2 * q)
    cos = math.cos(w0)
    root = 2 * math.sqrt(big) * alpha
    b = [
        big * ((big + 1) + (big - 1) * cos + root),
        -2 * big * ((big - 1) + (big + 1) * cos),
        big * ((big + 1) + (big - 1) * cos - root),
    ]
    a = [(big + 1) - (big - 1) * cos + root, 2 * ((big - 1) - (big + 1) * cos), (big + 1) - (big - 1) * cos - root]
    x = biquad(x, b, a)
    # High-pass at 38 Hz.
    w0 = 2 * math.pi * 38.0 / rate
    alpha = math.sin(w0) / (2 * 0.5)
    cos = math.cos(w0)
    b = [(1 + cos) / 2, -(1 + cos), (1 + cos) / 2]
    a = [1 + alpha, -2 * cos, 1 - alpha]
    return biquad(x, b, a)


def momentary(channels, rate, window=0.4):
    """The loudest `window` (400 ms: "momentary"), in LUFS (the sound padded with silence if shorter)."""
    weighted = [k_weight(c, rate) for c in channels]
    win = int(window * rate)
    hop = int(window / 4 * rate)
    length = max(len(weighted[0]), win)
    weighted = [np.pad(c, (0, length - len(c))) for c in weighted]
    best = -math.inf
    for start in range(0, length - win + 1, hop):
        power = sum(float(np.mean(c[start : start + win] ** 2)) for c in weighted)
        if power > 0:
            best = max(best, -0.691 + 10 * math.log10(power))
    return best


def measure(path):
    channels, rate = read(path)
    peak = 20 * math.log10(max(float(np.max(np.abs(c))) for c in channels))
    return momentary(channels, rate), peak


if __name__ == "__main__":
    for path in sys.argv[1:]:
        lufs, peak = measure(path)
        print(f"{lufs:7.2f} LUFS  peak {peak:6.2f} dBFS  {path}")
