# Changelog

All notable changes are recorded here. MailBox Ultra follows [Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-04-29

**BREAKING CHANGE.** MailBox Ultra is now a native macOS app, not a CLI binary with a browser UI.

- Ships as `MailBoxUltra.app` inside a `.dmg`. Drag into `/Applications`, right-click → Open on first launch.
- All configuration moves into a native Preferences window (`⌘,` or the gear button in the toolbar). The CLI flags are gone.
- HTML email previews are rendered by the system's `WKWebView` — the same engine Mail.app uses — embedded inside the app window. JavaScript is disabled and external link clicks open in your default browser.
- Native settings, native menus, native toolbar, native keyboard shortcuts (j / k / g / / / 1–6 / p / d / ⇧⌘X / t / ⌘, / ?).
- The `--update` self-update path is gone; updates come from the GitHub releases page.
- The HTTP API, SSE stream, and embedded vanilla-JS UI are removed. The capture core (SMTP server, MIME parser, ring-buffer store, optional NDJSON logging, optional upstream relay) is unchanged and still ~88 tests deep.

Linux and Windows builds are not produced.

## [0.2.0] - 2026-04-29

- HTML preview now has Desktop / iPad / Mobile size buttons so you can see how a captured email reflows at different widths. Selection is remembered across messages.
- HTML preview iframe fills the full height of the detail pane.
- Fixed: clicking a message in the list sometimes left the detail pane on the placeholder instead of showing the email.

## [0.1.0] - 2026-04-28

Initial release. Bind a port and catch every email your app tries to send. No real delivery, no setup. SMTP server, MIME parser, live web UI, JSON API, NDJSON log file, optional upstream relay.
