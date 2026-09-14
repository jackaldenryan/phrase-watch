from __future__ import annotations

from pathlib import Path

import numpy as np
import sherpa_onnx

from phrasewatch.config import AppConfig
from phrasewatch.paths import kws_files, vad_path, whisper_files


def create_vad() -> sherpa_onnx.VoiceActivityDetector:
    config = sherpa_onnx.VadModelConfig()
    config.silero_vad.model = str(vad_path())
    config.silero_vad.min_silence_duration = 0.25
    config.silero_vad.min_speech_duration = 0.15
    config.silero_vad.threshold = 0.5
    config.sample_rate = 16000
    return sherpa_onnx.VoiceActivityDetector(config, buffer_size_in_seconds=30)


def create_kws(cfg: AppConfig, keywords_file: Path) -> sherpa_onnx.KeywordSpotter:
    files = kws_files()
    return sherpa_onnx.KeywordSpotter(
        tokens=str(files["tokens"]),
        encoder=str(files["encoder"]),
        decoder=str(files["decoder"]),
        joiner=str(files["joiner"]),
        num_threads=cfg.num_threads,
        keywords_file=str(keywords_file),
        keywords_score=cfg.kws_score,
        keywords_threshold=cfg.kws_threshold,
        provider="cpu",
    )


def create_asr(cfg: AppConfig) -> sherpa_onnx.OfflineRecognizer:
    files = whisper_files()
    return sherpa_onnx.OfflineRecognizer.from_whisper(
        encoder=str(files["encoder"]),
        decoder=str(files["decoder"]),
        tokens=str(files["tokens"]),
        num_threads=cfg.num_threads,
        language="en",
        task="transcribe",
        tail_paddings=200,
    )


def transcribe(recognizer: sherpa_onnx.OfflineRecognizer, samples: np.ndarray, sample_rate: int = 16000) -> str:
    if samples.size == 0:
        return ""
    stream = recognizer.create_stream()
    stream.accept_waveform(sample_rate, samples.astype(np.float32, copy=False))
    recognizer.decode_stream(stream)
    return (stream.result.text or "").strip()


def kws_hits_from_wav(
    kws: sherpa_onnx.KeywordSpotter,
    samples: np.ndarray,
    sample_rate: int,
) -> list[str]:
    stream = kws.create_stream()
    stream.accept_waveform(sample_rate, samples.astype(np.float32, copy=False))
    tail = np.zeros(int(0.66 * sample_rate), dtype=np.float32)
    stream.accept_waveform(sample_rate, tail)
    stream.input_finished()
    hits: list[str] = []
    while kws.is_ready(stream):
        kws.decode_stream(stream)
        result = kws.get_result(stream)
        if result:
            hits.append(result)
            kws.reset_stream(stream)
    return hits
