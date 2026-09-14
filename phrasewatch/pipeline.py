from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

import numpy as np

from phrasewatch.audio import RingBuffer, read_wave
from phrasewatch.config import AppConfig
from phrasewatch.engines import create_asr, create_kws, create_vad, kws_hits_from_wav, transcribe
from phrasewatch.keywords import build_keywords_file
from phrasewatch.matcher import Debouncer, match_phrase
from phrasewatch.paths import LOG_PATH, ensure_support_dir


@dataclass
class Hit:
    phrase: str
    source: str
    transcript: str
    t: float


class PhrasePipeline:
    def __init__(
        self,
        cfg: AppConfig,
        on_hit: Callable[[Hit], None] | None = None,
    ) -> None:
        self.cfg = cfg
        self.on_hit = on_hit
        phrases = cfg.normalized_phrases()
        if not phrases:
            raise ValueError("no phrases configured")
        self.phrases = phrases
        keywords = build_keywords_file(phrases)
        self.kws = create_kws(cfg, keywords)
        self.kws_stream = self.kws.create_stream()
        self.vad = create_vad()
        self.asr = create_asr(cfg) if cfg.confirm_with_asr else None
        self.ring = RingBuffer(cfg.sample_rate, seconds=4.0)
        self.debouncer = Debouncer(seconds=cfg.debounce_seconds)
        self._window = int(0.1 * cfg.sample_rate)

    def feed(self, samples: np.ndarray, sample_rate: int | None = None) -> list[Hit]:
        sr = sample_rate or self.cfg.sample_rate
        x = samples.astype(np.float32, copy=False)
        self.ring.push(x)
        hits: list[Hit] = []

        self.kws_stream.accept_waveform(sr, x)
        while self.kws.is_ready(self.kws_stream):
            self.kws.decode_stream(self.kws_stream)
            raw = self.kws.get_result(self.kws_stream)
            if not raw:
                continue
            self.kws.reset_stream(self.kws_stream)
            clip = self.ring.last(2.8)
            transcript = raw
            source = "kws"
            if self.asr is not None and clip.size > 0:
                transcript = transcribe(self.asr, clip, self.cfg.sample_rate)
                source = "kws+asr"
                matched = match_phrase(transcript, self.phrases) or match_phrase(raw, self.phrases)
            else:
                matched = match_phrase(raw, self.phrases)
                if matched is None:
                    matched = raw
            if matched is None:
                continue
            if not self.debouncer.allow(matched):
                continue
            hit = Hit(phrase=matched, source=source, transcript=transcript, t=time.time())
            hits.append(hit)
            self._emit(hit)
        return hits

    def finish(self) -> list[Hit]:
        sr = self.cfg.sample_rate
        tail = np.zeros(int(0.66 * sr), dtype=np.float32)
        return self.feed(tail, sr)

    def process_wav(self, path: str | Path) -> list[Hit]:
        samples, sr = read_wave(path)
        if sr != self.cfg.sample_rate:
            samples = _resample_linear(samples, sr, self.cfg.sample_rate)
            sr = self.cfg.sample_rate
        hits: list[Hit] = []
        step = self._window
        for i in range(0, len(samples), step):
            hits.extend(self.feed(samples[i : i + step], sr))
        hits.extend(self.finish())
        return hits

    def process_wav_offline(self, path: str | Path) -> list[Hit]:
        samples, sr = read_wave(path)
        kws_found = kws_hits_from_wav(self.kws, samples, sr)
        transcript = ""
        if self.asr is not None:
            if sr != self.cfg.sample_rate:
                samples_16 = _resample_linear(samples, sr, self.cfg.sample_rate)
            else:
                samples_16 = samples
            transcript = transcribe(self.asr, samples_16, self.cfg.sample_rate)
        hits: list[Hit] = []
        matched = match_phrase(transcript, self.phrases) if transcript else None
        if matched is None:
            for raw in kws_found:
                matched = match_phrase(raw, self.phrases)
                if matched:
                    break
                matched = raw if raw else None
        if matched and self.debouncer.allow(matched):
            source = "kws+asr" if transcript else "kws"
            hit = Hit(
                phrase=matched,
                source=source,
                transcript=transcript or (kws_found[0] if kws_found else matched),
                t=time.time(),
            )
            hits.append(hit)
            self._emit(hit)
        return hits

    def _emit(self, hit: Hit) -> None:
        if self.cfg.log_hits:
            ensure_support_dir()
            rec = {
                "t": hit.t,
                "phrase": hit.phrase,
                "source": hit.source,
                "transcript": hit.transcript if self.cfg.log_hits else "",
            }
            with LOG_PATH.open("a", encoding="utf-8") as f:
                f.write(json.dumps(rec) + "\n")
        if self.on_hit:
            self.on_hit(hit)


def _resample_linear(samples: np.ndarray, src: int, dst: int) -> np.ndarray:
    if src == dst:
        return samples
    n_src = samples.shape[0]
    n_dst = int(round(n_src * dst / src))
    if n_src == 0 or n_dst == 0:
        return np.zeros(0, dtype=np.float32)
    x_old = np.linspace(0.0, 1.0, n_src, endpoint=False)
    x_new = np.linspace(0.0, 1.0, n_dst, endpoint=False)
    return np.interp(x_new, x_old, samples).astype(np.float32)
