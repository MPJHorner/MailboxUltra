#!/usr/bin/env bash
# MailBox Ultra one-liner installer for macOS.
#
#   curl -sSL https://raw.githubusercontent.com/MPJHorner/MailboxUltra/main/scripts/install.sh | bash
#
# Detects arch, downloads the matching .dmg from the latest GitHub release,
# mounts it, copies MailBox Ultra.app into /Applications, ejects, and clears
# the quarantine flag so first launch doesn't need a right-click → Open.
#
# Override the install location with $MBU_INSTALL_DIR (default /Applications).
#
# This is best-effort: if anything fails, fall back to the manual install
# at https://mpjhorner.github.io/MailboxUltra/install/

set -euo pipefail

REPO="MPJHorner/MailboxUltra"
RELEASES="https://github.com/${REPO}/releases"

err() { printf '\033[0;31m%s\033[0m\n' "$*" >&2; }
say() { printf '\033[0;32m%s\033[0m\n' "$*"; }
note() { printf '\033[0;36m%s\033[0m\n' "$*"; }

require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "missing dependency: $1"
        exit 1
    fi
}

require curl
require uname
require hdiutil

# ── Detect platform ──────────────────────────────────────────────
os="$(uname -s)"
if [[ "$os" != "Darwin" ]]; then
    err "MailBox Ultra is a native macOS app and only ships for Darwin."
    err "Detected OS: $os"
    err "If you want a cross-platform local SMTP catcher, try Mailpit:"
    err "  https://github.com/axllent/mailpit"
    exit 1
fi

arch="$(uname -m)"
case "$arch" in
    arm64|aarch64)  target="aarch64-apple-darwin" ;;
    x86_64|amd64)   target="x86_64-apple-darwin" ;;
    *)
        err "Unsupported arch: $arch"
        err "Falling back to the universal build."
        target="universal"
        ;;
esac

# ── Resolve latest version ───────────────────────────────────────
say "==> Detecting latest release"
version="$(curl -sSL -o /dev/null -w '%{url_effective}' "${RELEASES}/latest" | sed 's|.*/v||')"
if [[ -z "$version" ]]; then
    err "Could not determine latest release. Check ${RELEASES} manually."
    exit 1
fi
say "    Latest: v${version}"
say "    Target: ${target}"

# ── Download ─────────────────────────────────────────────────────
artefact="MailBoxUltra-${version}-${target}.dmg"
url="${RELEASES}/download/v${version}/${artefact}"
install_dir="${MBU_INSTALL_DIR:-/Applications}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp" 2>/dev/null || true' EXIT

say "==> Downloading ${artefact}"
if ! curl -fSL --progress-bar -o "${tmp}/${artefact}" "${url}"; then
    if [[ "$target" != "universal" ]]; then
        note "    per-arch dmg not available, retrying with universal"
        artefact="MailBoxUltra-${version}-universal.dmg"
        url="${RELEASES}/download/v${version}/${artefact}"
        curl -fSL --progress-bar -o "${tmp}/${artefact}" "${url}"
    else
        err "Download failed: ${url}"
        exit 1
    fi
fi

# ── Mount + copy ─────────────────────────────────────────────────
say "==> Mounting ${artefact}"
mountpoint="$(hdiutil attach "${tmp}/${artefact}" -nobrowse -noautoopen | awk '/Volumes/ {print $NF}')"
if [[ -z "$mountpoint" ]]; then
    err "Could not mount ${tmp}/${artefact}."
    err "Open it manually and drag MailBox Ultra.app into /Applications."
    exit 1
fi
trap 'hdiutil detach -quiet "${mountpoint}" 2>/dev/null || true; rm -rf "$tmp" 2>/dev/null || true' EXIT

# Find the app inside the mounted volume — pattern matches the build script's
# output ("MailBox Ultra.app", with a space).
app_in_dmg="$(find "${mountpoint}" -maxdepth 1 -name '*.app' -type d | head -1)"
if [[ -z "$app_in_dmg" ]]; then
    err "No .app bundle found inside the mounted DMG."
    exit 1
fi

say "==> Installing $(basename "$app_in_dmg") to ${install_dir}"
mkdir -p "${install_dir}"
target_app="${install_dir}/$(basename "$app_in_dmg")"
if [[ -d "$target_app" ]]; then
    note "    replacing existing $target_app"
    rm -rf "$target_app"
fi
cp -R "$app_in_dmg" "$target_app"

# Clear quarantine so Gatekeeper doesn't block first launch. Without this
# the user has to right-click → Open the first time. With it, they can
# double-click straight away. The "unsigned developer" dialog still appears
# once because the build is unsigned — but that prompt is one click, not two.
xattr -dr com.apple.quarantine "$target_app" 2>/dev/null || true

say ""
say "✓ Installed: $target_app"
say ""
note "Launch:  open '$target_app'"
note "Or:      Spotlight → 'MailBox Ultra'"
note ""
note "First launch: macOS may say the developer is unverified — click Open."
note "Default SMTP: smtp://127.0.0.1:1025  ·  Preferences: ⌘,"
