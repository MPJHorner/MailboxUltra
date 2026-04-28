---
title: "Relay mode"
description: "Forward every captured message to a real upstream SMTP server. Capture and deliver in the same hop."
slug: relay
---

# Relay mode

Plain capture is great for tests. Sometimes you want capture *and* delivery in the same run — for example, while staging a transactional rollout where you want to read every message before it ships, but still want it to ship.

`--relay` does that.

## Quick start

```sh
mailbox-ultra --relay smtp://relay.example.com:25
```

The capture port behaves exactly as before. After a message is captured and stored, a background relay task picks it up and reships it to `relay.example.com:25` over plain SMTP. The upstream's response does not affect what the original sender sees: capture always returns `250`. A relay failure is logged at `warn` level and the message stays in the in-memory buffer.

## With authentication

```sh
mailbox-ultra --relay smtp://alice:s3cret@relay.example.com:587
```

User-info in the URL becomes `AUTH PLAIN` credentials when the relay reaches the upstream. The credentials are not stored anywhere on disk, and the `/api/relay` endpoint returns the URL with the credentials redacted.

## TLS

`smtps://relay.example.com:465` is reserved for a future release. Right now it returns a clear error so you know the upstream isn't being silently downgraded:

```text
smtps:// relay is not yet implemented; use smtp:// or open a tracking issue
```

If your upstream only accepts TLS, use a side-car (`stunnel`, an SMTP proxy, or `ssmtp`) until TLS lands. Subscribe to the [issue tracker](https://github.com/MPJHorner/MailboxUltra/issues) for status.

## Runtime control

The relay can be enabled, edited, or disabled while the server is running, without a restart. The web UI's "Relay" pill in the top bar opens a dialog; under the hood it talks to the [`/api/relay` endpoints]({{base}}/api/#api-relay).

```sh
# Enable
curl -X PUT http://127.0.0.1:8025/api/relay \
  -H 'content-type: application/json' \
  -d '{"url":"smtp://relay.example.com:25"}'

# Inspect
curl http://127.0.0.1:8025/api/relay

# Disable
curl -X DELETE http://127.0.0.1:8025/api/relay
```

## One-off release

The Release tab on each captured message resends a single message to a specific upstream without touching the global relay setting. Useful when you want to capture mail in bulk and selectively forward only the ones you care about.

## Failure semantics

| Failure | Capture status | Relay status | Sender sees |
|---|---|---|---|
| Upstream refuses connection | Captured | Logged | `250` |
| Upstream rejects MAIL FROM | Captured | Logged | `250` |
| Upstream returns 5xx after DATA | Captured | Logged | `250` |
| MailBox Ultra is killed mid-relay | Captured (in buffer until shutdown) | Lost | `250` |

The capture path always wins. If you want the original sender to see relay failures, use a real MTA — that's a different tool.
