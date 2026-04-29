# CLAUDE.md

Notes for AI assistants (and humans) working on this repo.

## Versioning convention

MailBox Ultra follows [Semantic Versioning](https://semver.org/) and conventional commits.

When you commit a change that ships to users, do all four of these in the same commit:

1. Bump `version` in `Cargo.toml` (and let `cargo build` refresh `Cargo.lock`):
   - `feat:` prefix or `[minor]` -> minor (`0.x.0`).
   - `fix:` / no prefix or `[patch]` -> patch (`0.0.x`).
   - `[major]` or `BREAKING CHANGE:` -> major (`x.0.0`).
2. Add a top entry to `CHANGELOG.md`. Keep it terse, user-facing, and not overly technical -- describe what changed, not how.
3. Run the full check before pushing:
   ```sh
   make check         # fmt + clippy + tests
   ```
4. After the commit lands on `main`, create and push the matching tag. The release workflow does the rest.
   ```sh
   git tag "v$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
   git push --tags
   ```

`.github/workflows/release.yml` watches for `v*` tag pushes and builds binaries for macOS (Intel + Apple Silicon), Linux (x86_64 + aarch64), and Windows (x86_64), publishes a GitHub release, and uploads the archives + sha256 sums.

Documentation-only commits do not need a version bump.

## Test coverage

The project aims for high coverage on the testable surface. Two regions are excluded from coverage, both locally (`make coverage`) and in CI (`codecov.yml` + the `--ignore-filename-regex` flag). Each carries a header comment explaining its exemption:

- `src/main.rs` -- eframe + tokio boot shim. Cannot be deterministically driven from a unit test runner.
- `src/gui/**` -- egui rendering code. Immediate-mode UI, no clean headless surface; the data structures it consumes (`settings`, `server`, `store`, `message`) are all tested independently.

When a feature *can* be tested it must be -- exclusions are for code that physically can't be exercised, not for skipping work. If you find yourself wanting to add a file to the ignore list, justify it in that file's header comment first.

Run `make coverage` for a summary, `make coverage-html` for a per-line report. New code should land covered.

## Where things live

- `src/main.rs` -- eframe boot shim plus the tokio runtime spawn. Coverage-exempt.
- `src/lib.rs` -- public module list.
- `src/smtp.rs` -- SMTP listener, command parser, session state machine, AUTH PLAIN / LOGIN.
- `src/message.rs` -- captured `Message` type and MIME parsing helpers.
- `src/store.rs` -- bounded ring buffer + broadcast channel of new messages.
- `src/relay.rs` -- optional upstream relay task and `RelayConfig::from_url`.
- `src/settings.rs` -- persistent `PersistentSettings` (atomic JSON load/save, schema-versioned). Replaces the old CLI flags.
- `src/server.rs` -- `ServerHandle` lifecycle (start / restart / shutdown). Hot-updates the relay; full restart for SMTP / store changes; preserves captured messages across restarts.
- `src/gui/` -- egui front-end. Toolbar, inbox list, detail tabs, Preferences / Relay / Help windows, toast notifications, theme. Coverage-exempt.
  - `src/gui/native_html.rs` -- macOS-only `WKWebView` embedded in the eframe window for HTML email previews. Uses `objc2` + `objc2-web-kit`.
- `tools/icon-gen.rs` -- small build-tool binary (feature `icon-tool`) that rasterises `icon/icon.svg` into the per-size PNGs `iconutil` expects.
- `icon/` -- source SVG, generated `AppIcon.iconset`, generated `AppIcon.icns`. The `.icns` is committed.
- `assets/icon-512.png` -- the runtime window icon embedded into the binary via `include_bytes!`.
- `mac/` -- `Info.plist` template plus `build-app.sh`, `build-dmg.sh`, `build-app-universal.sh`. Hand-rolled bundling, no `cargo-bundle` dep.
- `tests/` -- integration tests against `ServerHandle` + `PersistentSettings`. Use `lettre` and a stub upstream SMTP server.
- `site/` -- the GitHub Pages docs site (handwritten static-site builder, deployed by `.github/workflows/site.yml`).

## Style

- No em dashes, no AI-slop adjectives ("blazing-fast", "beautiful", etc.) in user-facing text.
- README leads with the SEO-friendly description and screenshot, then a tight Install / Quick Start / Configuration / Shortcuts flow.
- README badges always point at `releases/latest`, so they update automatically when a new tag ships.

## How to run while developing

- `make run` — `cargo run` debug build. Fastest iteration, no .app shell.
- `make app && open target/aarch64-apple-darwin/release/MailBoxUltra.app` — build a real .app bundle and launch it like a Mac install. Tests dock-icon, window persistence, WKWebView attachment.
- `./scripts/simulate.py` — fires every realistic scenario at the running app (transactional, marketing, attachments, calendar invite, unicode, dark-mode-aware HTML, …). `--list` shows them all. `burst -n 200` for ring-buffer stress tests. Stdlib only.
- `make check` — pre-commit gate (fmt, clippy, tests). Same thing CI runs.

## macOS bundling

- `make icon` regenerates `icon/AppIcon.icns` from `icon/icon.svg`. The `.icns` is committed; rerun the target only when the SVG changes.
- `make app` produces the `.app` for the host arch. `make app-universal` produces a universal binary.
- `make dmg` (after `make app`) packages the bundle into a `.dmg` with an `/Applications` symlink for drag-to-install.
- Code-signing is gated on `APPLE_CERT_ID` in the environment; without it, builds are unsigned and require a right-click → Open on first launch (documented in `mac/README.md`).
