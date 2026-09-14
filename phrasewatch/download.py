from __future__ import annotations

import shutil
import tarfile
import urllib.request
from pathlib import Path

from phrasewatch.paths import KWS_DIRNAME, MODELS_DIR, WHISPER_DIRNAME, VAD_NAME


KWS_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/"
    "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2"
)
WHISPER_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/"
    "sherpa-onnx-whisper-tiny.en.tar.bz2"
)
VAD_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/"
    "silero_vad.onnx"
)


def _download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    print(f"Downloading {url}")
    urllib.request.urlretrieve(url, tmp)
    tmp.replace(dest)


def _extract_tar(archive: Path, dest_dir: Path, expected_root: str) -> None:
    dest_dir.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:bz2") as tar:
        tar.extractall(dest_dir)
    extracted = dest_dir / expected_root
    if not extracted.exists():
        kids = [p for p in dest_dir.iterdir() if p.is_dir()]
        if len(kids) == 1:
            kids[0].rename(extracted)


def download_models(force: bool = False) -> None:
    MODELS_DIR.mkdir(parents=True, exist_ok=True)

    vad = MODELS_DIR / VAD_NAME
    if force or not vad.exists():
        _download(VAD_URL, vad)

    kws_root = MODELS_DIR / KWS_DIRNAME
    encoder = kws_root / "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx"
    if force or not encoder.exists():
        archive = MODELS_DIR / f"{KWS_DIRNAME}.tar.bz2"
        _download(KWS_URL, archive)
        if kws_root.exists() and force:
            shutil.rmtree(kws_root)
        _extract_tar(archive, MODELS_DIR, KWS_DIRNAME)
        archive.unlink(missing_ok=True)

    whisper_root = MODELS_DIR / WHISPER_DIRNAME
    marker = whisper_root / "tiny.en-encoder.int8.onnx"
    marker2 = whisper_root / "tiny.en-encoder.onnx"
    if force or not (marker.exists() or marker2.exists()):
        archive = MODELS_DIR / f"{WHISPER_DIRNAME}.tar.bz2"
        _download(WHISPER_URL, archive)
        if whisper_root.exists() and force:
            shutil.rmtree(whisper_root)
        _extract_tar(archive, MODELS_DIR, WHISPER_DIRNAME)
        archive.unlink(missing_ok=True)

    print(f"Models ready in {MODELS_DIR}")


if __name__ == "__main__":
    download_models()
