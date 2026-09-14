import numpy as np

from phrasewatch.audio import RingBuffer


def test_ring_keeps_last_seconds() -> None:
    buf = RingBuffer(sample_rate=10, seconds=1.0)
    buf.push(np.arange(25, dtype=np.float32))
    last = buf.last(1.0)
    assert last.shape[0] == 10
    assert np.allclose(last, np.arange(15, 25, dtype=np.float32))
