# PhraseWatch

Fully local Mac menu-bar app. It listens on **your microphone only**, detects phrases you choose (default: “I’m sorry”), and shows a notification. Audio never leaves this computer.

Built for **macOS 26 + Apple Silicon** (tested on MacBook Pro, M1 Pro).

## How it works

1. **Silero VAD** — ignore silence  
2. **sherpa-onnx keyword spotting** (~3M English Zipformer) — watch up to ~20 custom phrases  
3. **Whisper tiny.en** (local) — confirm the hit so TV / similar words don’t false-alarm  

No Apple Speech, no cloud STT, no analytics.

## Setup

```bash
cd phrase-watch
uv venv --python 3.13
source .venv/bin/activate
uv pip install -e ".[dev]"
phrasewatch download-models
pytest
```

Models (~90 MB) download from [k2-fsa/sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) GitHub releases into `./models` (gitignored). After that, Airplane Mode is fine.

## Run

Terminal listener:

```bash
phrasewatch listen
```

Menu-bar app (no Dock icon, orange mic indicator stays on):

```bash
./scripts/install_app.sh
open ~/Applications/PhraseWatch.app
```

Or: `phrasewatch app`

## Phrases

```bash
phrasewatch phrases
phrasewatch phrases "my bad" "i apologize"
phrasewatch phrases --replace "i'm sorry" "i am sorry"
```

Config lives at `~/Library/Application Support/PhraseWatch/config.json`.

## Privacy

- Microphone only. No network after model download.
- No audio files written by default. Optional hit log is local JSONL.
- Quit from the menu-bar icon. The system mic dot is intentional.

## License

MIT. Bundled models keep their upstream licenses (sherpa-onnx Apache-2.0, Silero VAD MIT, Whisper code MIT).
