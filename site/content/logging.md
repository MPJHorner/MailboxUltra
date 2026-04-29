---
title: "Logging"
description: "Tail every captured message into NDJSON. Pair with relay to give an AI assistant a live feed."
slug: logging
---

# Logging

MailBox Ultra can append every captured message as one JSON object per line to a file you choose. Tail it from a script, hand it to a coding agent watching alongside you, or grep it after the fact.

## Turn it on

1. Open Preferences with `⌘,`.
2. Scroll to the **Logging** section.
3. Tick **Append every captured message as NDJSON to a log file**.
4. Click **Browse…** to pick a path, or type one in. Anything writable, e.g. `/tmp/mail.ndjson`.
5. Click **Apply**.

The log writer hot-swaps — the SMTP listener is not restarted. From this point on, every captured message is also serialised to a single line in your chosen file.

```sh
tail -f /tmp/mail.ndjson | jq '.subject, .envelope_to'
```

The file is created if missing, never truncated, and line-flushed after every message so a `tail -f` consumer (or an AI assistant) sees data immediately.

## What's in each line

```json
{
  "id": "2025-04-29T08:42:11.123Z-0001",
  "received_at": "2025-04-29T08:42:11.123Z",
  "envelope_from": "app@example.com",
  "envelope_to": ["dev@example.com"],
  "from": { "name": "Acme Billing", "address": "billing@acme.test" },
  "to": [{ "name": null, "address": "dev@example.com" }],
  "subject": "Your invoice for April",
  "headers": [["From", "Acme Billing <billing@acme.test>"], ["To", "dev@example.com"]],
  "text": "Hi Dev, your April invoice is attached.",
  "html": "<!doctype html>…",
  "attachments": [
    { "filename": "invoice-april.pdf", "content_type": "application/pdf", "size": 92341 }
  ],
  "size": 96012,
  "authenticated": false
}
```

| Field | Type | Notes |
|---|---|---|
| `id` | string | Stable identifier for this captured message. |
| `received_at` | string | RFC 3339 UTC timestamp. |
| `envelope_from` | string | The `MAIL FROM` value, empty for null sender. |
| `envelope_to` | string[] | Every `RCPT TO` recipient, in arrival order. |
| `from`, `to` | object / object[] | Parsed `From` / `To` header(s). `name` may be `null`. |
| `subject` | string | Decoded `Subject` header. |
| `headers` | [string, string][] | Every header in arrival order, including duplicates. |
| `text`, `html` | string \| null | The respective MIME alternatives, decoded. |
| `attachments` | object[] | Metadata only — filename, content_type, size in bytes. |
| `size` | number | Raw RFC 822 byte length of the captured message. |
| `authenticated` | boolean | True if the SMTP session was authenticated when the message was sent. |

Attachment **contents** are not in the log line — you can save them from the message's Attachments tab in the app.

## Pairing with relay

Combining Logging with Relay is the typical pattern when you want capture, real delivery, *and* a tail-able feed for an external observer (CI bot, AI agent, dashboard):

- Tick **Forward each captured message upstream** under Relay.
- Tick **Append every captured message as NDJSON to a log file** under Logging.
- Click **Apply**.

A relay failure is independent of the log: even if the upstream is down, the NDJSON entry is still written.

## Rotation

The log file is append-only. MailBox Ultra never rotates it. Use `logrotate`, `cronolog`, or just `truncate -s 0 /tmp/mail.ndjson` between sessions if it gets unwieldy. Or change the path under Preferences → Logging and click Apply — the writer rolls over to the new file immediately.
