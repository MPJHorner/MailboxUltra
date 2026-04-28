---
title: "Comparison"
description: "MailBox Ultra vs Mailpit, MailHog, Mailcatcher, Mailtrap. What overlaps, what doesn't."
slug: comparison
---

# Comparison

The local-SMTP-fake-inbox space has prior art. None of these tools are bad — MailBox Ultra exists because we wanted a Rust-native, single-binary version with a UI tuned for keyboard navigation, an SSE-first API, and tighter relay integration.

| Feature | MailBox Ultra | Mailpit | MailHog | Mailcatcher | Mailtrap |
|---|---|---|---|---|---|
| Language | Rust | Go | Go | Ruby | SaaS |
| Single binary | ✓ | ✓ | ✓ | (gem) | n/a |
| Default ports | 1025 / 8025 | 1025 / 8025 | 1025 / 8025 | 1025 / 1080 | n/a |
| HTML preview (sandboxed) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Plain-text view | ✓ | ✓ | ✓ | ✓ | ✓ |
| Header table | ✓ | ✓ | ✓ | ✓ | ✓ |
| Raw RFC 822 source | ✓ | ✓ | ✓ | ✓ | ✓ |
| Attachment downloads | ✓ | ✓ | ✓ | ✓ | ✓ |
| AUTH PLAIN / LOGIN | ✓ | ✓ | ✓ | ✓ | ✓ |
| STARTTLS | (planned) | ✓ | ✗ | ✗ | ✓ |
| Capture-and-relay upstream | ✓ | ✓ | ✓ (ReleaseConfig) | ✗ | ✓ |
| One-off Release per message | ✓ (UI + API) | ✓ | ✓ | ✓ | ✓ |
| Live event stream (SSE) | ✓ | ✓ (WebSocket) | ✗ | ✗ | n/a |
| NDJSON to file | ✓ | ✗ | ✗ | ✗ | n/a |
| NDJSON on stdout | ✓ | ✗ | ✗ | ✗ | n/a |
| Runs offline | ✓ | ✓ | ✓ | ✓ | ✗ |
| Telemetry | none | none | none | none | yes |
| Persistent storage | (planned) | ✓ (sqlite) | ✗ | ✓ (sqlite) | ✓ |
| IMAP / POP3 access | ✗ | ✓ | ✗ | ✗ | ✓ |
| MIT licensed | ✓ | ✓ | ✓ | ✓ | proprietary |

## When to use which

- **MailBox Ultra** — Rust shop, want a single statically-linked binary, value the SSE+NDJSON dual API, like keyboard-driven UIs.
- **Mailpit** — most feature-rich free option; if you need IMAP, POP3, or persistence today, this is the tool.
- **MailHog** — was the de-facto choice for years; still works fine but unmaintained for a while.
- **Mailcatcher** — Ruby projects already on the gem, simple needs, fine forever.
- **Mailtrap** — SaaS, team workflows, audit logs, regulatory needs.

## What's not on the roadmap

- IMAP / POP3 server. The captured-mail-as-real-mailbox use case is well served by Mailpit; if you need it, use Mailpit. MailBox Ultra optimises for the *inspect-and-relay* workflow.
- A persistent disk store. The buffer is a ring; messages survive across reloads of the web UI but not across `mailbox-ultra` restarts. Plan: optional sqlite-backed store, Q3.
- A hosted SaaS version.

## Migrating from Mailpit / MailHog

MailBox Ultra binds the same ports by default (1025 SMTP / 8025 HTTP), so anything pointing at those just works. The JSON API shape is different — see the [API reference]({{base}}/api/) — and the SSE endpoint replaces Mailpit's WebSocket. The CLI flags are similar but not identical; see the [CLI reference]({{base}}/cli/).
