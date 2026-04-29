#!/usr/bin/env bash
# Build for both Apple Silicon and Intel, lipo-merge the two binaries
# into a universal Mach-O, and assemble a single .app that runs natively
# on both arches.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"

if [[ ! -f "$ROOT/icon/AppIcon.icns" ]]; then
  echo "error: icon/AppIcon.icns is missing. Run \`make icon\` first." >&2
  exit 1
fi

(cd "$ROOT" && cargo build --release --target aarch64-apple-darwin)
(cd "$ROOT" && cargo build --release --target x86_64-apple-darwin)

UNI_DIR="$ROOT/target/universal-apple-darwin/release"
APP="$UNI_DIR/MailBoxUltra.app"
mkdir -p "$UNI_DIR"
rm -rf "$APP"

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create \
  "$ROOT/target/aarch64-apple-darwin/release/mailbox-ultra" \
  "$ROOT/target/x86_64-apple-darwin/release/mailbox-ultra" \
  -output "$APP/Contents/MacOS/MailBoxUltra"

cp "$ROOT/icon/AppIcon.icns"    "$APP/Contents/Resources/AppIcon.icns"
sed "s/__VERSION__/$VERSION/g"  "$ROOT/mac/Info.plist" > "$APP/Contents/Info.plist"
printf 'APPL????'             >  "$APP/Contents/PkgInfo"

if [[ -n "${APPLE_CERT_ID:-}" ]]; then
  echo "code-signing with $APPLE_CERT_ID"
  codesign --force --options runtime --timestamp \
    --sign "$APPLE_CERT_ID" \
    "$APP/Contents/MacOS/MailBoxUltra"
  codesign --force --options runtime --timestamp \
    --sign "$APPLE_CERT_ID" \
    "$APP"
fi

# Also produce a universal DMG.
DMG="$UNI_DIR/MailBoxUltra-${VERSION}-universal.dmg"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/MailBox Ultra.app"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "MailBox Ultra" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
shasum -a 256 "$DMG" > "$DMG.sha256"

echo "built $APP and $DMG"
