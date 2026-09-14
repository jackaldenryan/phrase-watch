from phrasewatch.matcher import Debouncer, LiveWindow, match_phrase, normalize


def test_normalize_contraction() -> None:
    assert normalize("I'm sorry") == "i am sorry"
    assert normalize("I’M SORRY!") == "i am sorry"
    assert normalize("  I'm   sorry.  ") == "i am sorry"


def test_match_exact_and_variant() -> None:
    phrases = ["i'm sorry", "i am sorry"]
    assert match_phrase("yeah I'm sorry about that", phrases) == "i'm sorry"
    assert match_phrase("I am sorry", phrases) == "i'm sorry"


def test_match_prefers_longer_phrase() -> None:
    phrases = ["sorry", "i'm sorry"]
    assert match_phrase("I said I'm sorry", phrases) == "i'm sorry"


def test_no_match() -> None:
    assert match_phrase("the weather is nice today", ["i'm sorry"]) is None
    assert match_phrase("worry", ["i'm sorry"]) is None


def test_multiple_user_phrases() -> None:
    phrases = [
        "i'm sorry",
        "i apologize",
        "my bad",
        "excuse me",
        "i didn't mean that",
    ]
    assert match_phrase("oh my bad dude", phrases) == "my bad"
    assert match_phrase("I apologize for the delay", phrases) == "i apologize"
    assert match_phrase("hello world", phrases) is None


def test_debouncer() -> None:
    d = Debouncer(seconds=5)
    assert d.allow("i'm sorry", now=0) is True
    assert d.allow("i'm sorry", now=1) is False
    assert d.allow("my bad", now=1) is True
    assert d.allow("i'm sorry", now=6) is True


def test_live_window() -> None:
    w = LiveWindow(max_words=8)
    w.push_text("hello there")
    w.push_text("I am")
    assert w.match(["i'm sorry"]) is None
    w.push_text("sorry")
    assert w.match(["i'm sorry"]) == "i'm sorry"
