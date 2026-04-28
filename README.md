# MailBox Ultra

A local SMTP fake inbox for developers. Bind a port, point your app at it, and every email it tries to send is parsed, stored, and shown to you in real time -- in your terminal and a live web UI. Built in Rust, ships as a single binary, runs entirely on your machine.

[![CI](https://github.com/MPJHorner/MailboxUltra/actions/workflows/ci.yml/badge.svg)](https://github.com/MPJHorner/MailboxUltra/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/github/actions/workflow/status/MPJHorner/MailboxUltra/ci.yml?branch=main&label=tests)](https://github.com/MPJHorner/MailboxUltra/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/MPJHorner/MailboxUltra/branch/main/graph/badge.svg)](https://codecov.io/gh/MPJHorner/MailboxUltra)
[![Release](https://img.shields.io/github/v/release/MPJHorner/MailboxUltra?display_name=tag&sort=semver)](https://github.com/MPJHorner/MailboxUltra/releases/latest)
[![Docs](https://img.shields.io/badge/docs-mpjhorner.github.io-2dd4bf)](https://mpjhorner.github.io/MailboxUltra/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](https://mpjhorner.github.io/MailboxUltra/install/)

> **Full documentation: [mpjhorner.github.io/MailboxUltra](https://mpjhorner.github.io/MailboxUltra/)** · [Install](https://mpjhorner.github.io/MailboxUltra/install/) · [CLI reference](https://mpjhorner.github.io/MailboxUltra/cli/) · [Changelog](https://mpjhorner.github.io/MailboxUltra/changelog/)

![MailBox Ultra web UI](docs/screenshot.png)

## Why

A real SMTP relay in front of your dev environment is overkill, and a SaaS sandbox needs an account and an internet round-trip. MailBox Ultra is the local alternative. Point any sender (your Laravel app, Rails Mailers, Django EmailBackend, Node nodemailer, a shell `swaks`) at `localhost:1025` and every message is parsed, stored, and shown to you immediately. Nothing is delivered, nothing leaves your machine. Plain text, HTML, MIME multipart, attachments, headers, raw RFC 822 source -- all of it, in a view you can actually read.

## Install

### Pre-built binaries

Download the latest from the [releases page](https://github.com/MPJHorner/MailboxUltra/releases/latest):

```sh
# macOS, Apple Silicon
curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-aarch64-apple-darwin.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra
```

Linux, Intel Mac, and Windows archives ship alongside in the same release. Each archive includes a matching `.sha256`. Full instructions on the [install page](https://mpjhorner.github.io/MailboxUltra/install/).

### Cargo

```sh
cargo install --git https://github.com/MPJHorner/MailboxUltra
```

### From source

```sh
git clone https://github.com/MPJHorner/MailboxUltra.git
cd MailboxUltra
cargo build --release
./target/release/mailbox-ultra
```

## Quick start

```sh
mailbox-ultra
```

Defaults bind `127.0.0.1:1025` for SMTP and `127.0.0.1:8025` for the web UI. The banner prints what it actually bound:

```
  ✉  MailBox Ultra
    SMTP    smtp://127.0.0.1:1025
    Web UI  http://127.0.0.1:8025
```

Send a message from any client:

```sh
swaks --to dev@example.com --from app@example.com \
  --server 127.0.0.1:1025 \
  --header "Subject: Hello from MailBoxUltra" \
  --body "It works."
```

Open `http://127.0.0.1:8025` to inspect the HTML rendering, the plain text part, every header, attachments, and the full raw RFC 822 source.

The full tour, every flag, the JSON API, relay mode, NDJSON logging, keyboard shortcuts, and configuration reference all live on the [docs site](https://mpjhorner.github.io/MailboxUltra/).

## Documentation

- **[Install](https://mpjhorner.github.io/MailboxUltra/install/)** -- binaries, Cargo, source, checksums, troubleshooting.
- **[Quick start](https://mpjhorner.github.io/MailboxUltra/quick-start/)** -- your first captured email, end to end.
- **[CLI reference](https://mpjhorner.github.io/MailboxUltra/cli/)** -- every flag, with examples.
- **[SMTP](https://mpjhorner.github.io/MailboxUltra/smtp/)** -- supported commands, AUTH, size limits.
- **[Relay mode](https://mpjhorner.github.io/MailboxUltra/relay/)** -- forward captured mail to a real upstream MTA.
- **[Logging](https://mpjhorner.github.io/MailboxUltra/logging/)** -- NDJSON `--log-file`, AI-assistant pairing.
- **[Web UI](https://mpjhorner.github.io/MailboxUltra/web-ui/)** -- tabs, formatters, keyboard shortcuts.
- **[JSON API](https://mpjhorner.github.io/MailboxUltra/api/)** -- programmatic access + SSE stream.
- **[Configuration](https://mpjhorner.github.io/MailboxUltra/configuration/)** -- every knob in one table.
- **[Use cases](https://mpjhorner.github.io/MailboxUltra/use-cases/)** -- transactional mail, password resets, SDKs, AI pairing.
- **[Comparison](https://mpjhorner.github.io/MailboxUltra/comparison/)** -- vs Mailpit, MailHog, Mailtrap, Mailcatcher.
- **[Changelog](https://mpjhorner.github.io/MailboxUltra/changelog/)** -- every release.

## Contributing

Issues and pull requests are welcome. Please run `make check` before submitting a PR; if you're adding a feature, add a test next to it. See the [contributing page](https://mpjhorner.github.io/MailboxUltra/contributing/) for the full conventions, coverage policy, and release flow.

## License

[MIT](LICENSE) © 2026 MPJHorner.
