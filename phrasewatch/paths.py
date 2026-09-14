from __future__ import annotations

import os
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
MODELS_DIR = Path(os.environ.get("PHRASEWATCH_MODELS", PROJECT_ROOT / "models"))
SUPPORT_DIR = Path.home() / "Library" / "Application Support" / "PhraseWatch"
CONFIG_PATH = SUPPORT_DIR / "config.json"
LOG_PATH = SUPPORT_DIR / "hits.jsonl"

KWS_DIRNAME = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
WHISPER_DIRNAME = "sherpa-onnx-whisper-tiny.en"
VAD_NAME = "silero_vad.onnx"


def ensure_support_dir() -> Path:
    SUPPORT_DIR.mkdir(parents=True, exist_ok=True)
    return SUPPORT_DIR


def kws_dir() -> Path:
    return MODELS_DIR / KWS_DIRNAME


def whisper_dir() -> Path:
    return MODELS_DIR / WHISPER_DIRNAME


def vad_path() -> Path:
    return MODELS_DIR / VAD_NAME


def kws_files() -> dict[str, Path]:
    d = kws_dir()
    return {
        "encoder": d / "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
        "decoder": d / "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
        "joiner": d / "joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
        "tokens": d / "tokens.txt",
        "bpe": d / "bpe.model",
    }


def whisper_files() -> dict[str, Path]:
    d = whisper_dir()
    encoder = d / "tiny.en-encoder.int8.onnx"
    if not encoder.exists():
        encoder = d / "tiny.en-encoder.onnx"
    decoder = d / "tiny.en-decoder.int8.onnx"
    if not decoder.exists():
        decoder = d / "tiny.en-decoder.onnx"
    tokens = d / "tiny.en-tokens.txt"
    if not tokens.exists():
        tokens = d / "tokens.txt"
    return {"encoder": encoder, "decoder": decoder, "tokens": tokens}


def models_ready() -> tuple[bool, list[str]]:
    missing: list[str] = []
    if not vad_path().exists():
        missing.append(str(vad_path()))
    for name, path in kws_files().items():
        if name == "bpe":
            continue
        if not path.exists():
            missing.append(str(path))
    for path in whisper_files().values():
        if not path.exists():
            missing.append(str(path))
    return (not missing, missing)
