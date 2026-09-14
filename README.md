# PhraseWatch

Download the Mac app from [Releases](https://github.com/jackaldenryan/phrase-watch/releases/latest), drag it into Applications, and open it. It lives in the Dock and menu bar. When you say a phrase you chose (default: “I’m sorry”), it warns you. Audio stays on this Mac.

## Install

1. Open the [latest release](https://github.com/jackaldenryan/phrase-watch/releases/latest).
2. Download **PhraseWatch-x.y.z.dmg**.
3. Open the disk image and drag **PhraseWatch** into Applications.
4. Open **PhraseWatch** from Applications.

The app is not notarized yet. If macOS says it cannot be opened, right-click the app, choose Open, and confirm. You can also run this once:

```
xattr -cr "/Applications/PhraseWatch.app"
```

Then open it again from Applications.

A `.zip` is attached to the same release. You do not need it for a first install.

## First launch

1. Grant **Microphone**.
2. Wait while it downloads on-device models (Silero VAD and Whisper tiny.en). That happens once, from GitHub, into `~/Library/Application Support/PhraseWatch`. After that it works offline.
3. Click **Start listening**.
4. Say **I’m sorry**. You should get a notification.

## What to try after it is installed

- Add more phrases in the window, one per line, then **Save phrases**.
- Click **Stop listening**. Further speech should do nothing.
- Click **Start listening** again.
- Close the window. The app stays in the Dock and menu bar.
- Hits are stored on this Mac. The **Over time** chart can show today, weeks, or months, grouped by 15 minutes / hour / day / week / month. Filter **All phrases** or one phrase.

## Updates

The window has **Check for updates**. That looks at GitHub Releases. You do not need to download a new disk image after the first install unless you prefer to.

## Requirements

macOS 14 or newer, on Apple Silicon. No API key. Nothing is sent to the internet while listening.

## Build from source

This section is only for changing the app. Ordinary install is the release download above.

```
./scripts/package-app.sh
```

That writes `dist/PhraseWatch.app`, a zip, and a disk image.

Rust unit tests:

```
cd src-tauri && cargo test
```

To publish a version, set `VERSION`, commit, push to `origin/main`, then run:

```
./scripts/publish-tag.sh
```

GitHub Actions attaches the disk image and zip to the GitHub release. If the runner cannot build, package on a Mac and upload the files with `gh release create`.
