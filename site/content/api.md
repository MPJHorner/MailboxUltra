---
title: "JSON API"
description: "Programmatic access to captured messages and the live event stream."
slug: api
---

# JSON API

Every interaction the web UI does is a plain HTTP call against the same endpoints listed below. Hit them yourself from a script, a CI pipeline, or an AI assistant.

All endpoints accept any origin via permissive CORS, so you can build dashboards from elsewhere on your network during dev.

## `GET /api/health`

Liveness probe and basic introspection.

```sh
curl http://127.0.0.1:8025/api/health
```

```json
{
  "status": "ok",
  "version": "{{version}}",
  "count": 4,
  "smtp_port": 1025
}
```

## `GET /api/messages`

List captured messages, newest first.

```sh
curl 'http://127.0.0.1:8025/api/messages?limit=50'
```

`limit` defaults to 100, capped at 10 000. Each entry is the full structured message:

```json
{
  "id": "9b8b7d92-…",
  "received_at": "2026-04-28T12:34:56Z",
  "envelope_from": "alice@example.com",
  "envelope_to": ["bob@example.com"],
  "remote_addr": "127.0.0.1:54321",
  "authenticated": false,
  "from": { "name": "Alice", "address": "alice@example.com" },
  "to": [{ "name": null, "address": "bob@example.com" }],
  "cc": [],
  "subject": "Hello",
  "headers": [["From", "\"Alice\" <alice@example.com>"], ["Subject", "Hello"]],
  "text": "It works.",
  "html": "<p>It works.</p>",
  "attachments": [],
  "size": 320
}
```

The `headers` array preserves duplicates and original order — important for `Set-Cookie`, `Received`, and other headers that legitimately repeat.

## `DELETE /api/messages`

Clear every message in the buffer. Returns `204`.

## `GET /api/messages/{id}`

Fetch a single message by UUID. `404` if it was evicted from the ring buffer or never existed.

## `DELETE /api/messages/{id}`

Remove one message. Returns `204` on success, `404` if not found.

## `GET /api/messages/{id}/raw`

Returns the raw RFC 822 bytes exactly as the SMTP server received them. `Content-Type: message/rfc822`.

```sh
curl http://127.0.0.1:8025/api/messages/9b8b7d92-.../raw > captured.eml
```

## `GET /api/messages/{id}/attachments/{idx}`

Download a single attachment by zero-indexed position. The response sets `Content-Type` to the attachment's declared type and `Content-Disposition: attachment; filename="..."` when the part has a filename.

## `POST /api/messages/{id}/release`

Resend a single captured message to a target SMTP server.

```sh
curl -X POST http://127.0.0.1:8025/api/messages/9b8b7d92-.../release \
  -H 'content-type: application/json' \
  -d '{"smtp_url": "smtp://relay.example.com:25"}'
```

Returns `202 Accepted` when the relay handshake completed; `400` for malformed input, `502` if the upstream rejected.

## `GET / PUT / DELETE /api/relay` {#api-relay}

Read, set, and clear the global relay configuration at runtime.

```sh
# Read
curl http://127.0.0.1:8025/api/relay

# Set
curl -X PUT http://127.0.0.1:8025/api/relay \
  -H 'content-type: application/json' \
  -d '{"url":"smtp://relay.example.com:25","insecure":false}'

# Clear
curl -X DELETE http://127.0.0.1:8025/api/relay
```

The response always includes the active configuration with credentials redacted to `***@host:port` when AUTH is in use.

## `GET /api/stream`

[Server-Sent Events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events) stream of capture events. The web UI uses this for live updates.

| Event | Payload |
|---|---|
| `hello` | `{"version":"…"}` once per connection. |
| `message` | The full message JSON (same shape as `GET /api/messages/{id}`). |
| `cleared` | `{}` when the buffer was cleared. |
| `deleted` | `{"id":"…"}` when a single message was removed. |
| `resync` | `{}` if the server's broadcast channel lagged; client should refetch the list. |

A keep-alive comment is sent every 15 seconds. Reconnect with the standard `EventSource` retry behaviour; MailBox Ultra never throttles.

## Rate limits

There aren't any. This is a single-user dev tool that runs on localhost.
