---
title: "Logging"
description: "Tail every captured message into NDJSON. Pair with relay to give an AI assistant a live feed."
slug: logging
---

# Logging

Two ways to drive other tools off MailBox Ultra captures.

## NDJSON to a file

```sh
mailbox-ultra --log-file /tmp/mail.ndjson
```

One JSON object per line. The file is created if missing, never truncated, and `flush`ed after every message so a `tail -f` consumer (or an AI assistant) sees data immediately.

```sh
tail -f /tmp/mail.ndjson | jq '.subject, .envelope_to'
```

## NDJSON to stdout

```sh
mailbox-ultra --json
```

Same shape, but on stdout. The startup banner and the per-message printer are suppressed in `--json` mode so the stream stays strictly machine-readable.

Pipe it directly into a downstream tool:

```sh
mailbox-ultra --json | python -c 'import sys, json
for line in sys.stdin:
    m = json.loads(line)
    print(m["subject"], "->", m["envelope_to"])'
```

## Pairing with `--relay`

Combining `--log-file` with `--relay` is the typical pattern when you want capture, real delivery, and a tail-able feed for an external observer (CI bot, AI agent, dashboard).

```sh
mailbox-ultra \
  --relay smtp://relay.example.com:25 \
  --log-file /tmp/mail.ndjson
```

A relay failure is independent of the log. Even if the upstream is down, the NDJSON entry is still written.

## What's in each line

The full structured message: id, envelope, parsed addresses, subject, headers, text, html, attachment metadata, raw size. Attachment *contents* are not in the log line (use the `/api/messages/{id}/attachments/{idx}` endpoint to fetch those when you need them).

## Rotation

The log file is append-only. MailBox Ultra never rotates it. Use `logrotate`, `cronolog`, or just `truncate -s 0 /tmp/mail.ndjson` between sessions if it gets unwieldy.
