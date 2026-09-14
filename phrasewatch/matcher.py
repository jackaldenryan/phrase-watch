from __future__ import annotations

import re
import time
from dataclasses import dataclass, field


_APOS = re.compile(r"[\u2019']")
_NON_ALNUM = re.compile(r"[^a-z0-9\s]")
_SPACE = re.compile(r"\s+")
_IM = re.compile(r"\bi[\u2019']m\b")
_IAM = re.compile(r"\bi\s+am\b")


def normalize(text: str) -> str:
    t = text.lower().replace("\u2019", "'")
    t = _IM.sub("i am", t)
    t = _NON_ALNUM.sub(" ", t)
    t = _SPACE.sub(" ", t).strip()
    return t


def tokenize(text: str) -> list[str]:
    n = normalize(text)
    return n.split() if n else []


def match_phrase(transcript: str, phrases: list[str]) -> str | None:
    hay = f" {normalize(transcript)} "
    if hay == "  ":
        return None
    ordered = sorted(phrases, key=lambda p: len(normalize(p)), reverse=True)
    for phrase in ordered:
        needle = normalize(phrase)
        if not needle:
            continue
        if f" {needle} " in hay:
            return phrase
    return None


@dataclass
class Debouncer:
    seconds: float = 8.0
    _last: dict[str, float] = field(default_factory=dict)

    def allow(self, phrase: str, now: float | None = None) -> bool:
        t = time.monotonic() if now is None else now
        key = normalize(phrase)
        prev = self._last.get(key)
        if prev is not None and (t - prev) < self.seconds:
            return False
        self._last[key] = t
        return True


def sliding_contains(words: list[str], phrase_words: list[str]) -> bool:
    if not phrase_words or len(words) < len(phrase_words):
        return False
    n = len(phrase_words)
    for i in range(len(words) - n + 1):
        if words[i : i + n] == phrase_words:
            return True
    return False


class LiveWindow:
    def __init__(self, max_words: int = 24) -> None:
        self.max_words = max_words
        self.words: list[str] = []

    def push_text(self, text: str) -> None:
        self.words.extend(tokenize(text))
        if len(self.words) > self.max_words:
            self.words = self.words[-self.max_words :]

    def match(self, phrases: list[str]) -> str | None:
        hay = " ".join(self.words)
        return match_phrase(hay, phrases)
