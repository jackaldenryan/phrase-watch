#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="$(tr -d '[:space:]' < VERSION)"
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

if [[ ! -d node_modules ]]; then
  npm install
fi

npm run tauri build

bundle="$root/src-tauri/target/release/bundle"
app_src="$(find "$bundle/macos" -name "PhraseWatch.app" -maxdepth 2 | head -n 1)"
if [[ -z "$app_src" ]]; then
  echo "Tauri did not produce PhraseWatch.app"
  exit 1
fi

dist="$root/dist"
rm -rf "$dist"
mkdir -p "$dist"
ditto "$app_src" "$dist/PhraseWatch.app"

identity="${CODESIGN_IDENTITY:-}"
if [[ -n "$identity" ]]; then
  codesign --force --deep --sign "$identity" --identifier com.jackaldenryan.phrase-watch "$dist/PhraseWatch.app"
else
  codesign --force --deep --sign - --identifier com.jackaldenryan.phrase-watch "$dist/PhraseWatch.app" || true
fi

ditto -c -k --keepParent "$dist/PhraseWatch.app" "$dist/PhraseWatch-$version.zip"

dmg_root="$dist/dmg"
rm -rf "$dmg_root"
mkdir -p "$dmg_root"
ditto "$dist/PhraseWatch.app" "$dmg_root/PhraseWatch.app"
ln -s /Applications "$dmg_root/Applications"
hdiutil create \
  -volname "PhraseWatch" \
  -srcfolder "$dmg_root" \
  -ov \
  -format UDZO \
  "$dist/PhraseWatch-$version.dmg" >/dev/null
rm -rf "$dmg_root"

echo "$dist/PhraseWatch.app"
echo "$dist/PhraseWatch-$version.zip"
echo "$dist/PhraseWatch-$version.dmg"
