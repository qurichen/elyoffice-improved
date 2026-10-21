# Usage: python music.py, in a venv with numpy.
import wave
from pathlib import Path

import numpy as np

SR = 48000
BEAT = 60 / 112
BARS = 42
N = round(BARS * 4 * BEAT * SR)
OUT = Path(__file__).parent / "out" / "music.wav"
TARGET_LUFS = -16.0
CEILING = 10 ** (-1.0 / 20)

rng = np.random.default_rng(1120)
dry = np.zeros((N, 2))
send = np.zeros((N, 2))

# (bass root, pad voicing, arpeggio tones): D A Bm G
CHORDS = [
    (38, [50, 57, 62, 66, 69], [62, 66, 69, 74]),
    (45, [49, 57, 61, 64, 69], [61, 64, 69, 73]),
    (47, [47, 59, 62, 66, 71], [62, 66, 71, 74]),
    (43, [43, 59, 62, 67, 71], [62, 67, 71, 74]),
]
ARP = [0, 1, 2, 1, 3, 2, 1, 2]
INTRO = [(0, 74), (4, 73), (6, 76), (8, 78), (10, 74), (12, 79), (14, 78)]
COUNTER = [(0, 78, 2), (2, 76, 1), (3, 74, 1), (4, 76, 3), (7, 73, 1),
           (8, 74, 2), (10, 78, 2), (12, 79, 2), (14, 78, 1), (15, 76, 1)]


def at(beat):
    return round(beat * BEAT * SR)


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def times(sec):
    return np.arange(int(sec * SR)) / SR


def attack(sig, sec):
    a = max(1, int(sec * SR))
    sig[:a] *= np.linspace(0.0, 1.0, a)
    return sig


def tail(sig, sec=0.02):
    r = int(sec * SR)
    ramp = np.linspace(1.0, 0.0, r)
    sig[-r:] *= ramp if sig.ndim == 1 else ramp[:, None]
    return sig


def put(sig, beat, gain=1.0, pan=0.0, rev=0.2):
    i = at(beat)
    if sig.ndim == 1:
        th = (pan + 1) * np.pi / 4
        sig = np.stack([sig * np.cos(th), sig * np.sin(th)], 1) * np.sqrt(2)
    n = min(len(sig), N - i)
    dry[i:i + n] += sig[:n] * gain
    send[i:i + n] += sig[:n] * gain * rev


def band_noise(n, lo, hi):
    spec = np.fft.rfft(rng.standard_normal(n))
    f = np.fft.rfftfreq(n, 1 / SR)
    spec *= (f > lo) * (f < hi)
    out = np.fft.irfft(spec, n)
    return out / np.abs(out).max()


def pluck(m, dur=1.0, tau=0.45, partials=8):
    t = times(dur)
    s = np.zeros_like(t)
    for h in range(1, partials + 1):
        if hz(m) * h > 15000:
            break
        s += np.sin(2 * np.pi * hz(m) * h * t) * np.exp(-t * (1 + 0.8 * (h - 1)) / tau) / h ** 1.4
    return tail(attack(s * 0.5, 0.002))


def mallet(m, dur=1.6, tau=0.7):
    t = times(dur)
    f = hz(m)
    s = (np.sin(2 * np.pi * f * t) * np.exp(-t / tau)
         + 0.25 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / (tau / 3))
         + 0.12 * np.sin(2 * np.pi * 4.0 * f * t) * np.exp(-t / (tau / 8)))
    return tail(attack(s * 0.5, 0.003))


def tick(level):
    t = times(0.08)
    s = (np.sin(2 * np.pi * 1750 * t) * np.exp(-t / 0.009)
         + 0.5 * np.sin(2 * np.pi * 2950 * t) * np.exp(-t / 0.005)
         + 0.3 * band_noise(len(t), 2000, 8000) * np.exp(-t / 0.0015))
    return tail(attack(s * level, 0.0003), 0.01)


def kick(level=1.0, decay=0.22):
    t = times(0.7)
    f = 46 + 70 * np.exp(-t / 0.032)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / decay)
    return tail(attack(s * level, 0.0015))


def snap():
    t = times(0.3)
    e = sum(np.exp(-np.clip(t - d, 0, None) / 0.004) * (t >= d) for d in (0.0, 0.008, 0.016))
    e = e + 0.45 * np.exp(-t / 0.055)
    s = band_noise(len(t), 900, 5000) * e
    return tail(attack(s * 0.5, 0.0004))


def shaker():
    t = times(0.09)
    s = band_noise(len(t), 5000, 12000) * np.exp(-t / 0.022)
    return tail(attack(s * 0.3, 0.003))


def bass(m, dur):
    t = times(dur)
    w = 2 * np.pi * hz(m) * t
    s = (np.sin(w) + 0.28 * np.sin(2 * w) + 0.08 * np.sin(3 * w)) * np.exp(-t / 0.9)
    return tail(attack(np.tanh(1.4 * s) / np.tanh(1.4), 0.004), 0.04)


def pad(voices, sec, rise, release=0.6, decay=None):
    t = times(sec + release)
    out = np.zeros((len(t), 2))
    for m in voices:
        for side, cents in ((0, -6), (1, 6)):
            f = hz(m) * 2 ** (cents / 1200)
            ph = rng.uniform(0, 2 * np.pi, 4)
            for h, a in enumerate((1.0, 0.4, 0.18, 0.07), 1):
                out[:, side] += a * np.sin(2 * np.pi * f * h * t + ph[h - 1])
    env = np.ones(len(t))
    r = int(rise * SR)
    env[:r] = np.sin(np.linspace(0, np.pi / 2, r)) ** 2
    if decay:
        env *= np.exp(-t / decay)
    else:
        k = int(sec * SR)
        env[k:] *= np.cos(np.linspace(0, np.pi / 2, len(t) - k)) ** 2
    return tail(out * env[:, None] / len(voices) * 0.35, 0.05)


def bar_start(b):
    return b * 4


def intro(b):
    for beat in range(4):
        put(tick(1.0 if beat == 0 else 0.6), bar_start(b) + beat, 0.16, 0.0, 0.1)
    for beat, m in INTRO:
        if beat // 4 == b:
            put(pluck(m, 2.2, 0.9), beat, 0.55, 0.15, 0.35)


def build(b):
    root, voices, tones = CHORDS[b % 4]
    s = bar_start(b)
    for beat in range(4):
        put(tick(1.0 if beat == 0 else 0.6), s + beat, 0.14, 0.0, 0.1)
        put(pluck(tones[ARP[beat * 2]], 1.4, 0.6), s + beat, 0.38, 0.25 if beat % 2 else -0.25, 0.3)
    for beat in (0, 2):
        put(kick(0.55), s + beat, 0.8, 0.0, 0.03)
    put(pad(voices, 4 * BEAT, 1.6 if b == 4 else 0.25), s, 0.55 + 0.05 * (b - 4), rev=0.35)


def groove(b, lift, arp_up=0, sixteenths=False, four=False, counter=False):
    root, voices, tones = CHORDS[b % 4]
    s = bar_start(b)
    for beat in (range(4) if four else (0, 2)):
        put(kick(0.9), s + beat, 0.95, 0.0, 0.03)
    for beat in (1, 3):
        put(snap(), s + beat, 0.42 * lift, 0.05, 0.25)
    for beat in range(4):
        put(tick(0.7 if beat else 1.0), s + beat, 0.09, -0.1, 0.1)
    if lift > 1.0:
        for beat in range(4):
            put(shaker(), s + beat + 0.5, 0.2 * lift, 0.35, 0.1)
    step = 0.25 if sixteenths else 0.5
    for k in range(int(4 / step)):
        m = tones[ARP[k % 8]] + arp_up
        put(pluck(m, 0.9, 0.32), s + k * step, 0.33 * lift * (1.0 if k % 2 == 0 else 0.8),
            -0.35 if k % 2 else 0.35, 0.28)
    for off, dur, interval in ((0, 1.4, 0), (1.5, 0.45, 0), (2, 1.4, 0), (3.5, 0.45, 7)):
        put(bass(root + interval, dur * BEAT), s + off, 0.5, 0.0, 0.02)
    put(pad(voices, 4 * BEAT, 0.25), s, 0.42, rev=0.35)
    if counter:
        for beat, m, length in COUNTER:
            if beat // 4 == b % 4:
                put(mallet(m, length * BEAT + 0.8, 0.6), s + beat % 4, 0.42, 0.3, 0.4)


def interlude(b):
    root, voices, tones = CHORDS[b % 4]
    s = bar_start(b)
    put(pad(voices, 4 * BEAT, 0.6 if b == 32 else 0.25), s, 0.7, rev=0.45)
    put(pluck(tones[3], 2.4, 1.0), s, 0.45, 0.2, 0.45)
    put(pluck(tones[1], 2.0, 0.9), s + 2, 0.35, -0.2, 0.45)
    if b % 2:
        put(pluck(tones[2], 1.6, 0.8), s + 3, 0.28, 0.1, 0.45)


def ending():
    s = bar_start(40)
    put(kick(1.0, 0.45), s, 1.0, 0.0, 0.05)
    put(snap(), s, 0.35, 0.0, 0.4)
    put(bass(38, 3.6), s, 0.6, 0.0, 0.05)
    put(pad([50, 57, 62, 66, 69, 74], 4.2, 0.01, 0.0, decay=1.3), s, 0.9, rev=0.45)
    for m, pan in ((62, -0.3), (66, 0.3), (69, -0.15), (74, 0.15), (78, 0.0)):
        put(mallet(m, 4.2, 1.1), s, 0.32, pan, 0.45)
    put(pluck(86, 3.0, 1.2), s, 0.2, 0.0, 0.5)


def arrange():
    for b in range(40):
        if b < 4:
            intro(b)
        elif b < 8:
            build(b)
        elif b < 20:
            groove(b, 1.0)
        elif b < 32:
            groove(b, 1.12, arp_up=12, counter=True)
        elif b < 36:
            interlude(b)
        else:
            groove(b, 0.85 + 0.1 * (b - 36), sixteenths=b >= 38, four=b >= 38)
    ending()


def reverb(x):
    n_ir = int(2.4 * SR)
    t = np.arange(n_ir) / SR
    f = np.fft.rfftfreq(n_ir, 1 / SR)
    nfft = 1 << int(np.ceil(np.log2(N + n_ir)))
    out = np.zeros_like(x)
    for c in range(2):
        ir = np.fft.irfft(np.fft.rfft(rng.standard_normal(n_ir)) / (1 + (f / 5000) ** 2), n_ir)
        ir *= np.exp(-t / 0.32)
        ir = np.concatenate([np.zeros(int(0.015 * SR)), ir])[:n_ir]
        ir /= np.sqrt(np.sum(ir ** 2))
        out[:, c] = np.fft.irfft(np.fft.rfft(x[:, c], nfft) * np.fft.rfft(ir, nfft), nfft)[:N]
    return out


def biquad(f, b, a):
    z = np.exp(-2j * np.pi * f / SR)
    return (b[0] + b[1] * z + b[2] * z ** 2) / (a[0] + a[1] * z + a[2] * z ** 2)


def lufs(x):
    f = np.fft.rfftfreq(len(x), 1 / SR)
    k = (biquad(f, (1.53512485958697, -2.69169618940638, 1.19839281085285),
                (1.0, -1.69065929318241, 0.73248077421585))
         * biquad(f, (1.0, -2.0, 1.0), (1.0, -1.99004745483398, 0.99007225036621)))
    y = np.fft.irfft(np.fft.rfft(x, axis=0) * k[:, None], len(x), axis=0)
    c = np.concatenate([[0.0], np.cumsum(np.sum(y ** 2, 1))])
    starts = np.arange(0, len(x) - 19200 + 1, 4800)
    p = (c[starts + 19200] - c[starts]) / 19200
    p = p[-0.691 + 10 * np.log10(p + 1e-20) > -70]
    p = p[-0.691 + 10 * np.log10(p) > -0.691 + 10 * np.log10(p.mean()) - 10]
    return -0.691 + 10 * np.log10(p.mean())


def run_min(x, w):
    n = len(x)
    xp = np.concatenate([x, np.ones(-(-(n + w) // w) * w - n)]).reshape(-1, w)
    pre = np.minimum.accumulate(xp, 1).ravel()
    suf = np.minimum.accumulate(xp[:, ::-1], 1)[:, ::-1].ravel()
    return np.minimum(suf[:n], pre[w - 1:w - 1 + n])


def limit(x, half=480):
    need = np.minimum(1.0, CEILING / np.maximum(np.abs(x).max(1), 1e-12))
    hold = run_min(np.concatenate([np.ones(half), need, np.ones(half)]), 2 * half + 1)
    h = half // 2
    c = np.concatenate([[0.0], np.cumsum(np.concatenate([np.ones(h), hold[:N], np.ones(h)]))])
    gain = (c[2 * h + 1:] - c[:-2 * h - 1]) / (2 * h + 1)
    return np.clip(x * gain[:, None], -CEILING, CEILING)


def master():
    mix = dry + reverb(send) * 0.32
    fade = np.ones(N)
    k = int(1.4 * SR)
    fade[-k:] = np.cos(np.linspace(0, np.pi / 2, k)) ** 2
    mix *= fade[:, None]
    g = 1.0
    for _ in range(4):
        g *= 10 ** ((TARGET_LUFS - lufs(limit(mix * g))) / 20)
    return limit(mix * g)


def write(x):
    OUT.parent.mkdir(parents=True, exist_ok=True)
    pcm = np.round(x * 8388607).astype("<i4").view(np.uint8).reshape(-1, 4)[:, :3]
    with wave.open(str(OUT), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(3)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def main():
    arrange()
    out = master()
    write(out)
    print(f"{OUT}: {N / SR:.3f} s, peak {20 * np.log10(np.abs(out).max()):.2f} dBFS, "
          f"{lufs(out):.2f} LUFS")


if __name__ == "__main__":
    main()
