from __future__ import annotations

import re
from pathlib import Path

import sentencepiece as spm

from phrasewatch.matcher import normalize
from phrasewatch.paths import SUPPORT_DIR, ensure_support_dir, kws_files


def phrase_label(phrase: str) -> str:
    n = normalize(phrase).replace(" ", "_").upper()
    n = re.sub(r"[^A-Z0-9_]", "", n)
    return n or "PHRASE"


def raw_keyword_line(phrase: str) -> str:
    spoken = normalize(phrase).upper()
    return f"{spoken} @{phrase_label(phrase)}"


def write_raw_keywords(phrases: list[str], path: Path) -> None:
    lines = [raw_keyword_line(p) for p in phrases if normalize(p)]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def tokenize_phrase(sp: spm.SentencePieceProcessor, phrase: str) -> str:
    spoken = normalize(phrase).upper()
    pieces = sp.encode(spoken, out_type=str)
    return " ".join(pieces)


def build_keywords_file(phrases: list[str]) -> Path:
    ensure_support_dir()
    files = kws_files()
    raw = SUPPORT_DIR / "keywords_raw.txt"
    out = SUPPORT_DIR / "keywords.txt"
    write_raw_keywords(phrases, raw)
    sp = spm.SentencePieceProcessor()
    sp.load(str(files["bpe"]))
    lines = []
    for phrase in phrases:
        if not normalize(phrase):
            continue
        token_line = tokenize_phrase(sp, phrase)
        if not token_line:
            continue
        lines.append(f"{token_line} :1.5 #0.25 @{phrase_label(phrase)}")
    if not lines:
        raise RuntimeError("Could not tokenize keywords")
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return out
