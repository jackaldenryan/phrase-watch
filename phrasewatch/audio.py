from __future__ import annotations

import wave
from pathlib import Path

import numpy as np


def read_wave(path: str | Path) -> tuple[np.ndarray, int]:
    with wave.open(str(path)) as f:
        if f.getnchannels() != 1:
            raise ValueError("wave must be mono")
        if f.getsampwidth() != 2:
            raise ValueError("wave must be 16-bit")
        raw = f.readframes(f.getnframes())
        samples = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
        return samples, f.getframerate()


def write_wave(path: str | Path, samples: np.ndarray, sample_rate: int) -> None:
    clipped = np.clip(samples, -1.0, 1.0)
    pcm = (clipped * 32767.0).astype(np.int16)
    with wave.open(str(path), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(sample_rate)
        f.writeframes(pcm.tobytes())


class RingBuffer:
    def __init__(self, sample_rate: int, seconds: float = 4.0) -> None:
        self.sample_rate = sample_rate
        self.capacity = int(sample_rate * seconds)
        self._data = np.zeros(self.capacity, dtype=np.float32)
        self._n = 0

    def push(self, samples: np.ndarray) -> None:
        x = samples.astype(np.float32, copy=False)
        n = x.shape[0]
        if n >= self.capacity:
            self._data[:] = x[-self.capacity :]
            self._n = self.capacity
            return
        if self._n + n <= self.capacity:
            self._data[self._n : self._n + n] = x
            self._n += n
            return
        keep = self.capacity - n
        self._data[:keep] = self._data[self._n - keep : self._n]
        self._data[keep:] = x
        self._n = self.capacity

    def last(self, seconds: float) -> np.ndarray:
        want = min(int(self.sample_rate * seconds), self._n)
        if want <= 0:
            return np.zeros(0, dtype=np.float32)
        return self._data[self._n - want : self._n].copy()
