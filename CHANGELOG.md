# Changelog

All notable changes are recorded here. MailBox Ultra follows [Semantic Versioning](https://semver.org/).

## [2.0.0] - 2026-04-29

**BREAKING CHANGE.** MailBox Ultra is now a native macOS application — no CLI binary, no browser UI, no HTTP server. The 0.x line was a Rust CLI that bound an SMTP listener and an embedded vanilla-JS web UI on a separate port; everything was driven by `--flags`. 2.0 is a real `.app` bundle you drop into `/Applications`.

### What's new

- **Ships as `MailBoxUltra.app` inside a `.dmg`.** Drag into `/Applications`, right-click → Open on first launch (one-time Gatekeeper prompt). Universal binary; runs natively on Intel and Apple Silicon.
- **Native Preferences window** (`⌘,` or the gear button) replaces every `--flag`: SMTP port, bind address, hostname, max message size, AUTH PLAIN/LOGIN, ring-buffer size, upstream relay URL, NDJSON log file, theme. Atomic JSON persistence in `~/Library/Application Support/com.mpjhorner.MailBoxUltra/settings.json`. **Apply** restarts only the servers whose settings changed; captured messages survive across an SMTP restart.
- **HTML email rendered by the system `WKWebView`** — the same engine Mail.app uses — embedded inside the app window. JavaScript is disabled, link clicks intercepted and shelled to the default browser, captured email HTML sandboxed.
- **Faithful responsive preview.** The HTML tab's Desktop / iPad / Mobile buttons resize the WKWebView frame *and* swap the User-Agent (iOS Mail / iPad Mail), so `@media (max-width: …)` rules and any UA-gated CSS branch the way they would on a real phone.
- **Inbox sidebar** designed for glanceability: two-line rows with the From address (strong, ellipsis) and Subject (dim, ellipsis), relative timestamps ("now / 2m ago / 1h ago / 3d ago") right-aligned, attachment marker (📎N) on row 2 when applicable. Hover lifts to a soft accent bar; selection gets the full accent left edge.
- **Theme** built around an explicit four-step surface hierarchy (`BG → BG_ELEV → BG_ELEV2 → BG_SOFT`) with helper functions for body/muted/dim text and surface levels. Settings dialog uses heading-with-underline sections (no nested cards), primary buttons fill with the brand accent, focus rings are clean 1px borders.
- **Native menus and shortcuts:** `⌘,` Preferences · `⌘Q` Quit · `j` / `k` / `↓` / `↑` next / prev message · `g` / `G` newest / oldest · `/` focus search · `1`-`6` switch detail tab · `p` pause / resume · `d` delete · `⇧⌘X` clear all · `t` toggle theme · `?` shortcuts cheat sheet · `Esc` close dialog / blur search.
- **Detail tabs:** HTML (rendered) · Text · Headers (sortable) · Attachments (Save…) · Source (RFC 822 with syntax highlighting) · Release (resend any captured message to a target SMTP URL).
- **Hand-drawn icon** that pops on the macOS dock; source SVG in `icon/icon.svg`, baked into `AppIcon.icns` via `make icon`.
- **`scripts/simulate.py`** ships ~30 ready-to-fire scenarios across work tooling (Linear, GitHub, Figma, Google Docs, Slack-shaped notifications), transactional (Stripe payment, Apple App Store receipt, Google Calendar reminder), newsletters (Substack), and ecommerce (a fictional bikini brand "MARÉ" with six perfectly-curated responsive templates: welcome, drop, order, cart, sale, lookbook). Default firing order is interleaved to look like a plausible day's inbox. Stdlib-only Python; no `pip install` needed.

### What's removed

- **No CLI binary, no `--flags`** — every option is a Preferences field. `mailbox-ultra` is the .app's executable, not something you run from a terminal.
- **No web UI, no HTTP API, no SSE stream.** The axum/tower/rust-embed stack is gone. Anything that scraped `/api/messages` or watched the `/events` SSE stream needs to switch to the [NDJSON log file](https://mpjhorner.github.io/MailboxUltra/logging/) — same data, line-buffered, tail-friendly.
- **No more `--update` self-update.** Updates come from the GitHub releases page; download the new `.dmg`, drag the new `.app` into `/Applications`.
- **macOS only.** Linux and Windows builds are not produced. WKWebView is macOS-only and we don't want a degraded preview on other platforms.

### Migrating from 0.x

The SMTP port stays at `1025` by default, so anything pointing at `127.0.0.1:1025` keeps working without changes. Old `--flag` workflows map to Preferences fields one-to-one; open `⌘,`, set the field, click Apply. If you were piping `--json` (NDJSON to stdout) into a tail, switch to Preferences → Logging and tick "Append every captured message as NDJSON to a log file" — same schema.

## [0.2.0] - 2026-04-29

- HTML preview now has Desktop / iPad / Mobile size buttons so you can see how a captured email reflows at different widths. Selection is remembered across messages.
- HTML preview iframe fills the full height of the detail pane.
- Fixed: clicking a message in the list sometimes left the detail pane on the placeholder instead of showing the email.

## [0.1.0] - 2026-04-28

Initial release. Bind a port and catch every email your app tries to send. No real delivery, no setup. SMTP server, MIME parser, live web UI, JSON API, NDJSON log file, optional upstream relay.
