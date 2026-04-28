---
title: "CLI reference"
description: "Every command-line flag for MailBox Ultra, captured straight from --help."
slug: cli
---

# CLI reference

Every flag, every default. The output below is captured from `mailbox-ultra --help` at site build time, so it's always in sync with the version of the docs you're reading.

## `mailbox-ultra --help`

<div class="code-block"><span class="code-lang">text</span><button class="copy-btn" type="button" aria-label="Copy code">copy</button><pre><code>{{cli_help}}</code></pre></div>

## Common patterns

### Bind to a non-default port

```sh
mailbox-ultra -s 2525 -u 9090
```

### Listen on all interfaces

```sh
mailbox-ultra --bind 0.0.0.0
```

### Require AUTH PLAIN / LOGIN

```sh
mailbox-ultra --auth alice:s3cret
```

Your sender library must offer the matching credentials. If it doesn't, drop the flag — the unauthenticated default still captures everything.

### Capture *and* relay upstream

```sh
mailbox-ultra --relay smtp://relay.example.com:25
```

Each captured message is also handed to the upstream MTA. See [relay mode]({{base}}/relay/) for AUTH credentials, TLS, and failure semantics.

### Tail every message into NDJSON

```sh
mailbox-ultra --log-file /tmp/mail.ndjson
```

One JSON object per line, flushed after every message. Pair with `tail -f /tmp/mail.ndjson | jq` or feed it to an AI assistant.

### Open the web UI on startup

```sh
mailbox-ultra --open
```

### Disable the web UI entirely

```sh
mailbox-ultra --no-ui
```

CLI-only mode is handy when you only want the terminal stream or the NDJSON log file.

### Run as a structured event source

```sh
mailbox-ultra --json
```

Emits one JSON object per message to stdout. Combine with `--no-update-check` to silence the GitHub release check on offline machines.

## Configuration matrix

The full configuration matrix lives on the [configuration page]({{base}}/configuration/) — same flags, side-by-side with their defaults and what they affect.
