#!/usr/bin/env bash
# Assemble target/<TARGET>/release/MailBoxUltra.app from a release binary +
# the .icns + Info.plist template. Pass a Cargo target triple as $1; the
# default is the host arch.
#
# Code-signing is gated on the APPLE_CERT_ID env var. With it set, we run
# `codesign --options runtime --timestamp --sign "$APPLE_CERT_ID"` against
# the bundle. Without it, we ship unsigned and the user has to right-click
# → Open on first launch.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-$(rustc -vV | awk '/host:/ { print $2 }')}"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"

OUT_DIR="$ROOT/target/$TARGET/release"
APP="$OUT_DIR/MailBoxUltra.app"

if [[ ! -f "$ROOT/icon/AppIcon.icns" ]]; then
  echo "error: icon/AppIcon.icns is missing. Run \`make icon\` first." >&2
  exit 1
fi

(cd "$ROOT" && cargo build --release --target "$TARGET")

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$OUT_DIR/mailbox-ultra"     "$APP/Contents/MacOS/MailBoxUltra"
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

echo "built $APP ($(du -sh "$APP" | cut -f1))"
