from __future__ import annotations

import numpy as np
import pytest

from phrasewatch.audio import read_wave
from phrasewatch.config import AppConfig
from phrasewatch.engines import create_asr, create_kws, create_vad, kws_hits_from_wav, transcribe
from phrasewatch.keywords import build_keywords_file
from phrasewatch.matcher import match_phrase
from phrasewatch.paths import kws_dir
from phrasewatch.pipeline import PhrasePipeline

pytestmark = pytest.mark.models


def test_vad_detects_speech_not_silence(require_models, sorry_wav) -> None:
    vad = create_vad()
    speech, sr = read_wave(sorry_wav)
    silence = np.zeros(sr, dtype=np.float32)
    speech_flag = _vad_any(vad, speech, sr)
    vad2 = create_vad()
    silence_flag = _vad_any(vad2, silence, sr)
    assert speech_flag is True
    assert silence_flag is False


def test_asr_transcribes_sorry(require_models, sorry_wav) -> None:
    cfg = AppConfig()
    asr = create_asr(cfg)
    samples, sr = read_wave(sorry_wav)
    text = transcribe(asr, samples, sr)
    assert match_phrase(text, ["i'm sorry", "sorry"]) is not None, text


def test_asr_does_not_hallucinate_sorry_on_weather(require_models, weather_wav) -> None:
    cfg = AppConfig()
    asr = create_asr(cfg)
    samples, sr = read_wave(weather_wav)
    text = transcribe(asr, samples, sr)
    assert match_phrase(text, ["i'm sorry"]) is None, text


def test_kws_official_light_up(require_models) -> None:
    wav = kws_dir() / "test_wavs" / "0.wav"
    if not wav.exists():
        pytest.skip("upstream KWS test wav missing")
    keywords = kws_dir() / "test_wavs" / "test_keywords.txt"
    if not keywords.exists():
        keywords = kws_dir() / "keywords.txt"
    cfg = AppConfig()
    kws = create_kws(cfg, keywords)
    samples, sr = read_wave(wav)
    hits = kws_hits_from_wav(kws, samples, sr)
    joined = " ".join(hits).upper()
    assert "LIGHT" in joined


def test_pipeline_sorry_wav(require_models, sorry_wav) -> None:
    cfg = AppConfig(phrases=["i'm sorry", "i am sorry", "sorry"], debounce_seconds=0)
    pipe = PhrasePipeline(cfg)
    hits = pipe.process_wav_offline(sorry_wav)
    if not hits:
        hits = pipe.process_wav(sorry_wav)
    assert hits, "expected a hit on TTS 'I'm sorry'"
    assert match_phrase(hits[0].phrase, cfg.phrases) or match_phrase(hits[0].transcript, cfg.phrases)


def test_pipeline_negative_weather(require_models, weather_wav) -> None:
    cfg = AppConfig(phrases=["i'm sorry", "i am sorry"], debounce_seconds=0)
    pipe = PhrasePipeline(cfg)
    hits = pipe.process_wav_offline(weather_wav)
    if not hits:
        hits = pipe.process_wav(weather_wav)
    assert hits == []


def test_user_can_set_many_phrases(require_models) -> None:
    phrases = [
        "i'm sorry",
        "i am sorry",
        "my bad",
        "i apologize",
        "excuse me",
        "never mind",
        "i take that back",
        "forget it",
        "pardon me",
        "i didn't mean that",
    ]
    path = build_keywords_file(phrases)
    text = path.read_text(encoding="utf-8")
    assert text.strip()
    assert len(text.strip().splitlines()) >= 8


def _vad_any(vad, samples: np.ndarray, sr: int) -> bool:
    window = getattr(getattr(vad, "config", None), "silero_vad", None)
    size = 512
    detected = False
    for i in range(0, len(samples), size):
        chunk = samples[i : i + size]
        if chunk.shape[0] < size:
            chunk = np.pad(chunk, (0, size - chunk.shape[0]))
        vad.accept_waveform(chunk)
        if getattr(vad, "is_speech_detected", None):
            if vad.is_speech_detected():
                detected = True
        while not vad.empty():
            vad.front
            vad.pop()
            detected = True
    vad.flush()
    while not vad.empty():
        vad.pop()
        detected = True
    return detected
