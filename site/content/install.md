---
title: "Install MailBox Ultra"
description: "Download the universal .dmg, drag MailBox Ultra.app into /Applications, open it. Or build from source with make app."
slug: install
---

# Install

MailBox Ultra ships as a native macOS application bundle (`.app`) packaged inside a `.dmg` for drag-and-drop install. Universal builds run natively on both Intel and Apple Silicon.

> **Requires macOS 11 (Big Sur) or newer.** Windows and Linux builds are not produced.

## Recommended — download the .dmg

1. Go to the [latest release](https://github.com/MPJHorner/MailboxUltra/releases/latest).
2. Download `MailBoxUltra-{{version}}-universal.dmg` (or the per-arch DMG if you prefer).
3. Open the DMG and drag **MailBox Ultra.app** into the bundled `Applications` shortcut.
4. Eject the DMG. Find **MailBox Ultra** in `/Applications` or via Spotlight.

### First launch — Gatekeeper

The build is unsigned (no Apple Developer ID), so macOS will refuse to open it on the first double-click. Two ways through:

**Option A — right-click → Open** (one-time prompt):

> Right-click `MailBox Ultra.app` in `/Applications`, choose **Open**, click **Open** in the dialog. Subsequent launches just need a normal double-click.

**Option B — clear the quarantine flag** in Terminal (skips the prompt entirely):

```sh
xattr -d com.apple.quarantine /Applications/MailBox\ Ultra.app
```

Either way, the app launches, the SMTP server binds `127.0.0.1:1025`, and the toolbar shows the URL it picked.

### Verify the download

Each release ships a `.sha256` companion file. Verify before opening:

```sh
shasum -a 256 -c MailBoxUltra-{{version}}-universal.dmg.sha256
```

## Build from source

Requires Xcode Command Line Tools (`xcode-select --install`) and a recent Rust toolchain (`rustup` works).

```sh
git clone https://github.com/MPJHorner/MailboxUltra.git
cd MailboxUltra
make icon         # rasterise icon/icon.svg → AppIcon.icns (one-time, committed)
make app          # build target/<arch>/release/MailBoxUltra.app for the host
open target/aarch64-apple-darwin/release/MailBoxUltra.app
```

Other targets:

| Command | What you get |
| --- | --- |
| `make run` | Debug build via `cargo run`, no `.app` bundle. Fastest iteration. |
| `make app` | Host-arch `.app` bundle (Apple Silicon Mac → arm64; Intel Mac → x86_64). |
| `make app-arm` | Apple Silicon `.app` regardless of host. |
| `make app-x86` | Intel `.app` regardless of host. |
| `make app-universal` | Universal `.app` (lipo-merged Intel + Apple Silicon). |
| `make dmg` | Wraps the host `.app` in a `.dmg` with a drag-to-Applications symlink. |
| `make check` | Pre-commit gate: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. |

## Updating

There's no auto-update. To upgrade, download the new release `.dmg`, drag the new `.app` into `/Applications` (replacing the old one). Settings persist in `~/Library/Application Support/com.mpjhorner.MailBoxUltra/`.

## Uninstall

```sh
rm -rf "/Applications/MailBox Ultra.app"
rm -rf "$HOME/Library/Application Support/com.mpjhorner.MailBoxUltra"
```

The second line wipes saved Preferences. If you only want to reset the app to defaults but keep it installed, open Preferences and click **Reset to defaults**.

## Code-signing & notarisation

The public release is unsigned. If you have a Developer ID certificate and want signed local builds, set `APPLE_CERT_ID` in your environment before `make app` — the build script picks it up and runs `codesign` over the bundle.

## Next

[Quick start →]({{base}}/quick-start/)
