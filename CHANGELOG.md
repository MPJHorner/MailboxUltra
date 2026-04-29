# Changelog

All notable changes are recorded here. MailBox Ultra follows [Semantic Versioning](https://semver.org/).

## [1.1.0] - 2026-04-29

The polish release on top of 1.0.0 — visual refinements, faithful responsive HTML preview, a much richer simulator, and native-only documentation throughout.

- **Inbox sidebar** redesigned: two-line rows, relative timestamps ("now / 2m ago / 1h ago / 3d ago"), ellipsis-truncated From + Subject, narrower default width. Hover lifts to a soft accent bar; selection gets the full accent left edge plus a tinted fill.
- **Settings dialog** flattened: section headings with thin underlines instead of nested bordered cards, primary buttons filled with the brand accent, focus ring softened to a subtle 1px border instead of the previous accent glow.
- **Theme** rebuilt around an explicit four-step surface hierarchy (`BG → BG_ELEV → BG_ELEV2 → BG_SOFT`) with helper functions for body/muted/dim text and surface levels, so consumers don't repeat the dark/light branch.
- **HTML preview — Mobile / iPad device buttons now also swap the WKWebView's User-Agent** to match iOS Mail / iPad Mail. Width + UA together give a faithful preview for any responsive email that branches on either; the page reloads in place when the device changes.
- **Simulator (`scripts/simulate.py`)** got six "MARÉ" ecommerce templates (welcome, new collection drop, order confirmation, cart abandonment, flash sale, lookbook) with real Unsplash imagery, a wordmark rendered as inline-styled text, and `<style>` `@media (max-width:540px)` rules that genuinely stack columns and shrink padding on the Mobile (390px) preview. Plus eight everyday email scenarios in the visual style of their senders: Linear issue assigned, GitHub PR review request, Figma comment, Google Doc comment, Substack newsletter, Stripe payment received, Google Calendar 15-minute reminder, Apple App Store receipt. Default firing order is curated to look like a plausible day's inbox.
- **Documentation site** overhauled for native-only: removed the CLI, Web UI, and HTTP API pages; rewrote Quick start / Preferences / SMTP / Relay / Logging / Comparison / Use cases / Contributing to lean on the Preferences window; new homepage hero with the screenshot above the fold.
- **README** updated with all three new screenshots (dark inbox, light order confirmation, dark mobile preview) and pointers to the docs site.

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
