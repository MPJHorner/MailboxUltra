---
title: "Configuration"
description: "Every flag, every default, every knob in one table."
slug: configuration
---

# Configuration

Single-table reference for everything you can tune. The CLI flags are the only public configuration surface; there are no config files, no env vars, no profiles.

| Flag | Default | What it does |
|---|---|---|
| `-s, --smtp-port <PORT>` | `1025` | Port the SMTP server binds. Falls back to the next free port if busy. |
| `-u, --ui-port <PORT>` | `8025` | Port the web UI server binds. Same fallback behaviour. |
| `--bind <ADDR>` | `127.0.0.1` | Bind address for both servers. |
| `--hostname <NAME>` | `MailBoxUltra` | Hostname announced in the SMTP banner and EHLO response. |
| `--max-message-size <N>` | `26214400` (25 MiB) | Maximum DATA payload accepted before returning `552`. |
| `--buffer-size <N>` | `1000` | Number of messages held in memory. Oldest are evicted. |
| `--auth <USER:PASS>` | (off) | Require AUTH PLAIN / AUTH LOGIN with these credentials. |
| `--relay <URL>` | (off) | Forward each captured message to upstream SMTP. `smtp://...` or future `smtps://...`. |
| `--relay-insecure` | `false` | Reserved for the upcoming smtps relay path. |
| `--log-file <PATH>` | (off) | Append every captured message as one JSON line per message. |
| `--no-ui` | `false` | Disable the web UI server. CLI / log file only. |
| `--no-cli` | `false` | Disable the colored CLI output. |
| `--json` | `false` | Print each captured message as a JSON object on stdout (NDJSON). Conflicts with `--no-cli`. |
| `--open` | `false` | Open the web UI in your default browser on startup. |
| `-v, --verbose` | `false` | Verbose CLI: prints recipients, headers, and a body preview per message. |
| `--update` | `false` | One-shot: download the latest release from GitHub and replace the binary. |
| `--no-update-check` | `false` | Silence the silent startup check that asks GitHub for new releases. |

## Conflicts

- `--json` and `--no-cli` are mutually exclusive. `--no-cli` produces no stdout output; `--json` produces structured stdout.
- `--smtp-port` and `--ui-port` cannot be the same port (unless `--no-ui` is also set).

## Sane defaults you'll rarely need to change

The defaults are tuned to mirror Mailpit and MailHog so anything pre-configured for those tools just works. The only knobs most projects ever touch:

```sh
mailbox-ultra -s 2525 -u 9090     # alternate ports
mailbox-ultra --bind 0.0.0.0      # listen on all interfaces (devcontainer / docker)
mailbox-ultra --auth dev:dev      # quiet a sender that won't speak without AUTH
mailbox-ultra --open              # open the UI on launch
```
