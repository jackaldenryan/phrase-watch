from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from phrasewatch.paths import models_ready

FIXTURES = Path(__file__).parent / "fixtures"


def pytest_configure(config: pytest.Config) -> None:
    FIXTURES.mkdir(parents=True, exist_ok=True)


def _say_wav(text: str, dest: Path) -> Path:
    if dest.exists() and dest.stat().st_size > 1000:
        return dest
    aiff = dest.with_suffix(".aiff")
    subprocess.run(["say", "-o", str(aiff), text], check=True)
    subprocess.run(
        ["afconvert", "-f", "WAVE", "-d", "LEI16@16000", str(aiff), str(dest)],
        check=True,
    )
    aiff.unlink(missing_ok=True)
    return dest


@pytest.fixture(scope="session")
def sorry_wav() -> Path:
    return _say_wav("I'm sorry", FIXTURES / "im_sorry.wav")


@pytest.fixture(scope="session")
def weather_wav() -> Path:
    return _say_wav("The weather is nice today", FIXTURES / "weather.wav")


@pytest.fixture(scope="session")
def require_models() -> None:
    ready, missing = models_ready()
    if not ready:
        pytest.skip("models not downloaded: " + ", ".join(missing[:4]))
