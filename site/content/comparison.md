---
title: "Comparison"
description: "MailBox Ultra vs Mailpit, MailHog, MailCatcher, Mailtrap. What overlaps, what doesn't."
slug: comparison
---

# Comparison

The local-SMTP-fake-inbox space has prior art. None of these tools are bad. MailBox Ultra exists for one specific reason: we wanted a real native macOS application instead of a CLI that ships its own browser-based UI. If you're happy in a browser tab, [Mailpit](https://github.com/axllent/mailpit) is excellent and probably the right call.

| | MailBox Ultra | Mailpit | MailHog | MailCatcher | Mailtrap / Mailosaur |
|---|---|---|---|---|---|
| Distribution | macOS .app | Single binary, web UI | Single binary, web UI | Ruby gem, web UI | SaaS |
| Platforms | macOS only | Linux, macOS, Windows | Linux, macOS, Windows | Linux, macOS, Windows | (browser) |
| Default SMTP port | 1025 | 1025 | 1025 | 1025 | issued per inbox |
| Inspect UI | native window, WebKit HTML preview | browser tab | browser tab | browser tab | browser tab |
| Attachment downloads | Save… in detail tab | yes | yes | yes | yes |
| Raw RFC 822 source | yes | yes | yes | yes | yes |
| AUTH PLAIN / LOGIN | yes | yes | yes | yes | yes |
| STARTTLS | no (intentional, local-only) | yes | no | no | yes |
| Capture-and-relay upstream | yes (hot-swap) | yes | yes | no | yes |
| One-off Release per message | yes (Release tab) | yes | yes | yes | yes |
| Persistent storage | no | yes (sqlite) | no | yes (sqlite) | yes |
| IMAP / POP3 | no | yes | no | no | yes |
| Runs offline | yes | yes | yes | yes | no |
| Telemetry | none | none | none | none | account required |
| Maintained | actively | actively | quiet for years | yes | actively |
| License | MIT | MIT | MIT | MIT | proprietary |

## When to use which

- **MailBox Ultra** — you live on macOS, want a real Mac app with a dock icon and `⌘,` Preferences, render HTML through the system WebKit, and don't need IMAP / POP3 / persistence.
- **Mailpit** — most feature-rich free option. Cross-platform, persistent sqlite store, IMAP / POP3, polished web UI. If a browser tab works for you, this is the tool.
- **MailHog** — was the de-facto choice for years; still works but unmaintained for a while. New projects should pick Mailpit instead.
- **MailCatcher** — Ruby projects already on the gem, simple needs, fine forever.
- **Mailtrap / Mailosaur** — SaaS, team workflows, audit logs, regulatory needs. Requires an account and an internet round-trip.
- **Local MTA + cron** — heavyweight, but the right answer if you genuinely need full delivery semantics and bounce handling. Not what most dev environments need.

## What's not on the roadmap

- IMAP / POP3 server. The captured-mail-as-real-mailbox use case is well served by Mailpit; if you need it, use Mailpit. MailBox Ultra optimises for the inspect-and-relay workflow.
- Persistent disk storage. The buffer is a ring; messages survive across Preferences edits and SMTP restarts but not across app quits. Plan: optional sqlite-backed store, no firm date.
- A hosted SaaS version. MailBox Ultra is a local tool by design.
- Linux / Windows builds. The app is built around `WKWebView` for HTML rendering, the macOS Application Support directory for settings, and the Mac menu bar / dock model. Porting to other platforms would mean replacing all three. Not happening.

## Migrating from Mailpit / MailHog

The SMTP port is the same (1025), so anything pointing at `127.0.0.1:1025` keeps working. If your Mailpit / MailHog setup shipped its own browser UI on `:8025`, you can stop running that server — MailBox Ultra's UI is the .app window itself. Anything that scraped `/api/messages` or watched the WebSocket / SSE stream needs replacing; the [NDJSON log file]({{base}}/logging/) is the analogue.
