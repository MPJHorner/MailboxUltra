---
title: "Install"
description: "Install MailBox Ultra from a pre-built binary, Cargo, or source. macOS, Linux, Windows."
slug: install
---

# Install

MailBox Ultra ships as a single statically-linked binary on all three major desktop platforms. No runtime, no system services, no daemons.

## Pre-built binaries

The fastest path. Pick the archive that matches your platform from the [latest release](https://github.com/MPJHorner/MailboxUltra/releases/latest), extract it, and run the binary.

### macOS, Apple Silicon

```sh
curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-aarch64-apple-darwin.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra
```

### macOS, Intel

```sh
curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-x86_64-apple-darwin.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra
```

### Linux, x86_64

```sh
curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-x86_64-unknown-linux-gnu.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra
```

### Linux, ARM64

```sh
curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-aarch64-unknown-linux-gnu.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra
```

### Windows, x86_64

```powershell
Invoke-WebRequest -Uri https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-x86_64-pc-windows-msvc.zip -OutFile mailbox-ultra.zip
Expand-Archive mailbox-ultra.zip
./mailbox-ultra/mailbox-ultra.exe
```

### Verify the checksum

Each archive ships next to a matching `.sha256` file. Verify the download before running:

```sh
shasum -a 256 -c mailbox-ultra-aarch64-apple-darwin.tar.gz.sha256
```

## Cargo

```sh
cargo install --git https://github.com/MPJHorner/MailboxUltra
```

The crate has not yet been published to crates.io; this installs the latest commit on `main`.

## From source

```sh
git clone https://github.com/MPJHorner/MailboxUltra.git
cd MailboxUltra
cargo build --release
./target/release/mailbox-ultra
```

A Rust toolchain ≥ 1.85 is required. The repo ships a `rust-toolchain.toml` that pins the stable channel.

## Next

[Quick start →]({{base}}/quick-start/)
