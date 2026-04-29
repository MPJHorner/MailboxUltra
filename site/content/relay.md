---
title: "Relay mode"
description: "Forward every captured message to a real upstream SMTP server. Capture and deliver in the same hop."
slug: relay
---

# Relay mode

Plain capture is great for tests. Sometimes you want capture *and* delivery in the same run — for example, while staging a transactional rollout where you want to read every message before it ships, but still want it to ship.

Relay mode does that.

## Quick start

1. Open Preferences with `⌘,`.
2. Scroll to the **Relay** section.
3. Tick **Forward each captured message upstream**.
4. Paste the upstream URL into the **Upstream URL** field, e.g. `smtp://relay.example.com:25`.
5. Click **Apply**.

The capture port behaves exactly as before. After a message is captured, a background relay task picks it up and reships it to the upstream MTA. The upstream's response does not affect what the original sender sees: capture always returns `250`. A relay failure is surfaced visually (the toolbar relay pill turns red) and the message stays in the inbox.

The toolbar **Relay** pill goes from `Relay  off` to `Relay  relay.example.com:25` once enabled. Click the pill any time for a smaller standalone dialog that toggles relay without opening the full Preferences window.

## With authentication

Put the credentials in the URL:

```text
smtp://alice:s3cret@relay.example.com:587
```

The userinfo becomes `AUTH PLAIN` credentials when the relay reaches the upstream. The credentials are stored as part of the settings JSON in your user-only Application Support directory.

## TLS

Use `smtps://` for upstreams that require TLS:

```text
smtps://relay.example.com:465
```

The connection is wrapped in TLS using the system trust store. If your upstream serves a dev / staging certificate that isn't in the trust store, tick **Skip TLS certificate verification** under Relay. That option is intended for local development; do not point it at production traffic.

## Hot-update

Relay settings are picked up without restarting the SMTP listener. You can flip the relay on, change the upstream URL, or turn it off mid-session — the captured-mail buffer is untouched, in-flight SMTP transactions complete, and the next captured message uses the new relay config.

## One-off Release

The **Release** tab on each captured message resends a single message to a specific upstream without touching the global relay setting. Useful when you want to capture mail in bulk and selectively forward only the ones you care about.

## Capture-then-forward semantics

| Failure | Capture status | Relay status | Sender sees |
|---|---|---|---|
| Upstream refuses connection | Captured | Logged + toolbar pill | `250` |
| Upstream rejects MAIL FROM | Captured | Logged + toolbar pill | `250` |
| Upstream returns 5xx after DATA | Captured | Logged + toolbar pill | `250` |
| MailBox Ultra is killed mid-relay | Captured (in buffer until shutdown) | Lost | `250` |

The capture path always wins. If you want the original sender to see relay failures, use a real MTA — that's a different tool.
