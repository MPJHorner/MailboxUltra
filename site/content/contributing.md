---
title: "Contributing"
description: "How to contribute to MailBox Ultra: workspace layout, test policy, release flow."
slug: contributing
---

# Contributing

Thanks for poking at this. Issues and pull requests are welcome.

## Local checks

```sh
make check         # fmt + clippy + tests; same gate CI runs
make coverage      # llvm-cov summary, matches CI exclusions
make coverage-html # per-line HTML report at target/llvm-cov/html/index.html
make app           # build a Mac .app bundle for the host arch
make app-universal # universal lipo-merged bundle (Intel + Apple Silicon)
make dmg           # wrap the host .app in a drag-to-Applications .dmg
make icon          # rasterise icon/icon.svg into AppIcon.icns (rerun only when SVG changes)
```

`make check` is the same gate CI runs. PRs that fail it get sent back automatically.

## Test policy

Every feature ships with a test. Coverage is tracked on the testable surface. Two regions are excluded, both locally and in CI, with the reason documented in the file's header comment:

- `src/main.rs` — eframe + tokio boot shim. Cannot be deterministically driven from a unit test runner.
- `src/gui/**` — egui rendering code. Immediate-mode UI; no clean headless surface. The data structures it consumes (`settings`, `server`, `store`, `message`, `relay`) are all tested independently.

If a feature can be tested it must be. Exclusions are for code that physically cannot be exercised, not for skipping work.

When adding code, prefer:

- A unit test inside the same module (`mod tests` block at the bottom of the file) for pure logic.
- An integration test under `tests/` for anything that drives a real socket or the full server lifecycle. These tests use `lettre` to drive SMTP, a stub upstream server to assert relay behaviour, and `ServerHandle` / `MessageStore` / `PersistentSettings` directly — no GUI involved.

New code lands covered.

## Visual smoke tests

`scripts/simulate.py` fires every realistic mail scenario at the running app — transactional, marketing, attachments, calendar invites, unicode, dark-mode-aware HTML, and so on:

```sh
make run &           # debug build of the app
./scripts/simulate.py --list      # see every scenario
./scripts/simulate.py marketing   # send one
./scripts/simulate.py burst -n 200 # ring-buffer stress test
```

Stdlib only, no Python dependencies. Useful for eyeballing UI changes before opening a PR.

## Style

- No em dashes used cosmetically in user-facing text. Use them only when grammatically correct.
- No AI-slop adjectives ("blazing-fast", "beautiful", etc.) in user-facing text.
- README leads with the SEO-friendly description and the screenshot, then a tight Install / Quick Start / Configuration / Shortcuts flow.
- Inline comments explain *why*, not *what*. The reader can read the code.

## Versioning and release flow

[Semantic Versioning](https://semver.org/), conventional commits.

When you commit a user-visible change, do all four in the same commit:

1. Bump `version` in `Cargo.toml`. `feat:` → minor, `fix:` / no prefix → patch, `[major]` or `BREAKING CHANGE:` → major.
2. Add a top entry to `CHANGELOG.md`. Keep it terse and user-facing.
3. Run `make check`.
4. After merging to `main`, push the matching tag:

```sh
git tag "v$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
git push --tags
```

The release workflow watches for `v*` tags and builds the macOS `.app` bundles (Intel, Apple Silicon, universal), wraps each in a `.dmg`, publishes the GitHub release, and uploads the archives + `.sha256` sums. The site workflow redeploys the docs on every push to `main`.

## Where things live

- `src/main.rs` — eframe boot shim plus the tokio runtime spawn. Coverage-exempt.
- `src/lib.rs` — public module list.
- `src/smtp.rs` — SMTP listener, command parser, session state machine, AUTH PLAIN / LOGIN.
- `src/message.rs` — captured `Message` type and MIME parsing helpers.
- `src/store.rs` — bounded ring buffer + broadcast channel of new messages.
- `src/relay.rs` — optional upstream relay task and `RelayConfig::from_url`.
- `src/settings.rs` — persistent `PersistentSettings` (atomic JSON load/save, schema-versioned).
- `src/server.rs` — `ServerHandle` lifecycle (start / restart / shutdown).
- `src/gui/` — egui front-end. Toolbar, inbox list, detail tabs, Preferences / Relay / Help windows, toasts, theme. Coverage-exempt.
- `src/gui/native_html.rs` — `WKWebView` embedded in the eframe window for HTML email previews. Uses `objc2` + `objc2-web-kit`.
- `tools/icon-gen.rs` — small build-tool binary (feature `icon-tool`) that rasterises `icon/icon.svg` into the per-size PNGs `iconutil` expects.
- `mac/` — `Info.plist` template plus `build-app.sh`, `build-dmg.sh`, `build-app-universal.sh`. Hand-rolled bundling.
- `tests/` — integration tests against `ServerHandle` + `PersistentSettings`. `lettre` SMTP, stub upstream server.
- `site/` — this docs site (handwritten static-site builder, deployed by `.github/workflows/site.yml`).

## Code of conduct

Be kind. We have a zero-tolerance policy for harassment. Disagreements about technical decisions are welcome; ad-hominem attacks are not.
