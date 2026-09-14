from __future__ import annotations

from typing import Callable

import sounddevice as sd

from phrasewatch.config import AppConfig
from phrasewatch.pipeline import Hit, PhrasePipeline


def listen_microphone(
    cfg: AppConfig,
    on_hit: Callable[[Hit], None],
    stop_flag: Callable[[], bool] | None = None,
) -> None:
    pipeline = PhrasePipeline(cfg, on_hit=on_hit)
    block = 512

    def callback(indata, frames, time_info, status) -> None:
        if stop_flag and stop_flag():
            raise sd.CallbackStop()
        mono = indata[:, 0].copy()
        pipeline.feed(mono, cfg.sample_rate)

    with sd.InputStream(
        samplerate=cfg.sample_rate,
        channels=1,
        dtype="float32",
        blocksize=block,
        callback=callback,
    ):
        while True:
            if stop_flag and stop_flag():
                break
            sd.sleep(200)
