from __future__ import annotations

import argparse
import sys
from pathlib import Path

from phrasewatch.config import AppConfig, load_config, save_config
from phrasewatch.download import download_models
from phrasewatch.paths import CONFIG_PATH, MODELS_DIR, models_ready
from phrasewatch.pipeline import Hit, PhrasePipeline


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="phrasewatch")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("download-models", help="Download VAD, KWS, and Whisper tiny.en locally")
    sub.add_parser("check", help="Verify models and config")
    sub.add_parser("app", help="Run the Mac menu-bar app")

    p_listen = sub.add_parser("listen", help="Listen to the microphone in the terminal")
    p_listen.add_argument("--no-confirm", action="store_true")

    p_wav = sub.add_parser("wav", help="Run the pipeline on a 16-bit mono wav file")
    p_wav.add_argument("path")
    p_wav.add_argument("--no-confirm", action="store_true")

    p_phrases = sub.add_parser("phrases", help="Show or set phrases")
    p_phrases.add_argument("phrase", nargs="*")
    p_phrases.add_argument("--replace", action="store_true")

    args = parser.parse_args(argv)

    if args.cmd == "download-models":
        download_models()
        return 0
    if args.cmd == "check":
        return _check()
    if args.cmd == "app":
        from phrasewatch.app import run_app

        run_app()
        return 0
    if args.cmd == "listen":
        return _listen(no_confirm=args.no_confirm)
    if args.cmd == "wav":
        return _wav(args.path, no_confirm=args.no_confirm)
    if args.cmd == "phrases":
        return _phrases(args.phrase, replace=args.replace)
    return 1


def _require_models() -> int:
    ready, missing = models_ready()
    if ready:
        return 0
    print("Missing models:")
    for m in missing:
        print(f"  {m}")
    print("Run: phrasewatch download-models")
    return 2


def _check() -> int:
    cfg = load_config()
    print(f"config: {CONFIG_PATH}")
    print(f"models: {MODELS_DIR}")
    print(f"phrases: {cfg.normalized_phrases()}")
    print(f"confirm_with_asr: {cfg.confirm_with_asr}")
    code = _require_models()
    if code == 0:
        print("models: ok")
    return code


def _print_hit(hit: Hit) -> None:
    print(f"HIT  phrase={hit.phrase!r}  source={hit.source}  transcript={hit.transcript!r}", flush=True)


def _listen(no_confirm: bool) -> int:
    if _require_models():
        return 2
    cfg = load_config()
    if no_confirm:
        cfg.confirm_with_asr = False
    from phrasewatch.listen import listen_microphone
    from phrasewatch.notifier import notify

    def on_hit(hit: Hit) -> None:
        _print_hit(hit)
        if cfg.notify:
            notify(hit)

    print("Listening. Ctrl-C to stop. Mic stays local.", flush=True)
    try:
        listen_microphone(cfg, on_hit)
    except KeyboardInterrupt:
        print("\nStopped.")
    return 0


def _wav(path: str, no_confirm: bool) -> int:
    if _require_models():
        return 2
    cfg = load_config()
    if no_confirm:
        cfg.confirm_with_asr = False
    pipe = PhrasePipeline(cfg)
    hits = pipe.process_wav_offline(Path(path))
    if not hits:
        hits = pipe.process_wav(Path(path))
    if not hits:
        print("No phrase detected.")
        return 0
    for hit in hits:
        _print_hit(hit)
    return 0


def _phrases(values: list[str], replace: bool) -> int:
    cfg = load_config()
    if not values:
        for p in cfg.normalized_phrases():
            print(p)
        return 0
    if replace:
        cfg.phrases = values
    else:
        cfg.phrases = cfg.normalized_phrases() + values
        cfg.phrases = AppConfig(phrases=cfg.phrases).normalized_phrases()
    save_config(cfg)
    for p in cfg.normalized_phrases():
        print(p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
