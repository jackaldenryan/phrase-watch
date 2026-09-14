#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$HOME/Applications/PhraseWatch.app"
MACOS="$APP/Contents/MacOS"
RES="$APP/Contents/Resources"
mkdir -p "$MACOS" "$RES" "$HOME/Applications"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>PhraseWatch</string>
  <key>CFBundleDisplayName</key><string>PhraseWatch</string>
  <key>CFBundleIdentifier</key><string>local.phrasewatch.app</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>PhraseWatch</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>PhraseWatch listens on this Mac only to detect phrases you choose. Audio never leaves the device.</string>
</dict>
</plist>
EOF

cat > "$MACOS/PhraseWatch" <<EOF
#!/bin/bash
export PATH="/opt/homebrew/bin:/usr/local/bin:\$PATH"
cd "$ROOT"
exec "$ROOT/.venv/bin/phrasewatch" app
EOF
chmod +x "$MACOS/PhraseWatch"

echo "Installed $APP"
echo "Open it from ~/Applications or: open \"$APP\""
