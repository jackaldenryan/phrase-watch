from phrasewatch.config import AppConfig, load_config, save_config


def test_default_phrases() -> None:
    cfg = AppConfig()
    assert "i'm sorry" in cfg.normalized_phrases()


def test_roundtrip(tmp_path) -> None:
    path = tmp_path / "config.json"
    cfg = AppConfig(phrases=["my bad", "i'm sorry", "I'm sorry", ""])
    save_config(cfg, path)
    loaded = load_config(path)
    assert loaded.normalized_phrases() == ["my bad", "i'm sorry"]
    assert loaded.confirm_with_asr is True
