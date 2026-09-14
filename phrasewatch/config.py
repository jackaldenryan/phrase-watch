from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path

from phrasewatch.paths import CONFIG_PATH, ensure_support_dir


DEFAULT_PHRASES = ["i'm sorry", "i am sorry"]


@dataclass
class AppConfig:
    phrases: list[str] = field(default_factory=lambda: list(DEFAULT_PHRASES))
    confirm_with_asr: bool = True
    debounce_seconds: float = 8.0
    notify: bool = True
    log_hits: bool = True
    sample_rate: int = 16000
    kws_score: float = 1.5
    kws_threshold: float = 0.25
    num_threads: int = 2

    def normalized_phrases(self) -> list[str]:
        out: list[str] = []
        seen: set[str] = set()
        for raw in self.phrases:
            p = raw.strip()
            if not p:
                continue
            key = p.lower()
            if key in seen:
                continue
            seen.add(key)
            out.append(p)
        return out


def load_config(path: Path | None = None) -> AppConfig:
    p = path or CONFIG_PATH
    if not p.exists():
        cfg = AppConfig()
        save_config(cfg, p)
        return cfg
    data = json.loads(p.read_text(encoding="utf-8"))
    known = {f.name for f in AppConfig.__dataclass_fields__.values()}
    filtered = {k: v for k, v in data.items() if k in known}
    if "phrases" in filtered and not isinstance(filtered["phrases"], list):
        filtered.pop("phrases")
    return AppConfig(**filtered)


def save_config(cfg: AppConfig, path: Path | None = None) -> None:
    p = path or CONFIG_PATH
    ensure_support_dir()
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(asdict(cfg), indent=2) + "\n", encoding="utf-8")
