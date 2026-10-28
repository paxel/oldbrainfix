"""Writes Old Brain Fix's own alarm sound (#102).

A two-note chime, E5 then C6, each a sine with a quiet octave overtone,
a short attack and an exponential decay; mono 16-bit PCM at 44100 Hz,
peaking at -3 dBFS. Standard library only and deterministic, so the
committed file is what this script writes:

    python3 tool/alarm_sound.py android/app/src/main/res/raw/obf_alarm.wav
"""

import math
import struct
import sys
import wave

RATE = 44100
LENGTH = 1.2
ATTACK = 0.010
DECAY = 0.22
OVERTONE = 0.25
PEAK_DBFS = -3.0

# (frequency in Hz, start in seconds)
NOTES = [(659.25, 0.0), (1046.5, 0.35)]


def samples():
    """The chime as 16-bit integers."""
    count = int(RATE * LENGTH)
    raw = []
    for i in range(count):
        t = i / RATE
        value = 0.0
        for frequency, start in NOTES:
            s = t - start
            if s < 0:
                continue
            envelope = min(1.0, s / ATTACK) * math.exp(-s / DECAY)
            phase = 2 * math.pi * frequency * s
            value += envelope * (math.sin(phase) + OVERTONE * math.sin(2 * phase))
        raw.append(value)
    # The last milliseconds fade to silence, so the end does not click.
    fade = int(RATE * 0.05)
    for i in range(fade):
        raw[count - 1 - i] *= i / fade
    peak = max(abs(v) for v in raw)
    scale = 32767 * 10 ** (PEAK_DBFS / 20) / peak
    return [int(round(v * scale)) for v in raw]


def write(path):
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(b"".join(struct.pack("<h", s) for s in samples()))


if __name__ == "__main__":
    write(sys.argv[1] if len(sys.argv) > 1 else "obf_alarm.wav")
