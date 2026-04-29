#!/usr/bin/env bash
# Package target/<TARGET>/release/MailBoxUltra.app into a DMG with an
# /Applications symlink so the user can drag-to-install.
#
# Run after mac/build-app.sh. Pass a Cargo target triple as $1.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-$(rustc -vV | awk '/host:/ { print $2 }')}"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"

APP="$ROOT/target/$TARGET/release/MailBoxUltra.app"
DMG="$ROOT/target/$TARGET/release/MailBoxUltra-${VERSION}-${TARGET}.dmg"

if [[ ! -d "$APP" ]]; then
  echo "error: $APP is missing. Run \`make app\` (or mac/build-app.sh) first." >&2
  exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp -R "$APP" "$STAGE/MailBox Ultra.app"
ln -s /Applications "$STAGE/Applications"

rm -f "$DMG"
hdiutil create -volname "MailBox Ultra" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
shasum -a 256 "$DMG" > "$DMG.sha256"

echo "built $DMG"
