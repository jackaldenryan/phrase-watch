from __future__ import annotations

import subprocess

from phrasewatch.pipeline import Hit


def notify(hit: Hit) -> None:
    title = "PhraseWatch"
    body = f'You said “{hit.phrase}”'
    script = (
        f'display notification { _osa(body) } with title { _osa(title) } '
        f'sound name "Ping"'
    )
    subprocess.run(["osascript", "-e", script], check=False, capture_output=True)


def _osa(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'
