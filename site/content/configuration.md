---
title: "Preferences"
description: "Every section, every field of the MailBox Ultra Preferences window."
slug: configuration
---

# Preferences

Open with `⌘,` or click the gear icon in the toolbar. Edit values, click **Apply** to save and apply, or **Cancel** to discard. The button at the bottom-left is **Reset to defaults** — it rebuilds the form from defaults but doesn't apply until you click **Apply**.

The window is split into six sections: Servers, SMTP, Capture, Relay, Logging, Appearance.

## Servers

| Field | Default | What it does |
|---|---|---|
| **SMTP port** | `1025` | Port the SMTP server binds. If busy, the server walks forward to the next free port and the toolbar shows what it actually got. |
| **Bind address** | `127.0.0.1` | Interface the SMTP listener binds to. Set to `0.0.0.0` for devcontainer / docker hosts that need to reach the app from outside the loopback. |

Changes here trigger a full SMTP listener restart on **Apply**. Captured messages are preserved across the restart.

## SMTP

| Field | Default | What it does |
|---|---|---|
| **Hostname** | `MailBoxUltra` | What the server announces in the `220` greeting and the `250 hello` line. Some libraries pin to a specific hostname during testing. |
| **Max message size** | `25` MiB | Maximum DATA payload accepted before returning `552`. Mirrors `EHLO SIZE`. |
| **Require AUTH** | off | When ticked, advertises `AUTH PLAIN LOGIN` and rejects `MAIL FROM` until the client authenticates. |
| **User** / **Password** | empty | Credentials checked when **Require AUTH** is on. Stored as plaintext in the settings JSON; the file is in your user-only Application Support directory. |

See [the SMTP server page]({{base}}/smtp/) for protocol details.

## Capture

| Field | Default | What it does |
|---|---|---|
| **Buffer size** | `1000` messages | Number of captured messages held in memory. Once full, the oldest message is evicted as each new one arrives — a ring buffer. Increase if you batch-test mailers; decrease if you want a tighter visual list. |

Buffer size changes do not lose messages on Apply — the existing buffer is rebuilt at the new capacity, oldest first.

## Relay

| Field | Default | What it does |
|---|---|---|
| **Forward each captured message upstream** | off | Master toggle. When off, capture only. When on, every captured message is also relayed. |
| **Upstream URL** | empty | `smtp://host:port` for plain SMTP, `smtps://host:port` for TLS. Optional userinfo for AUTH PLAIN credentials: `smtp://user:pass@host:port`. |
| **Skip TLS certificate verification** | off | Only meaningful with `smtps://`. For dev / staging certs that aren't in the system trust store. Don't tick this against production. |

Relay settings hot-update — the SMTP listener does not restart when relay changes. The toolbar **Relay** pill also has its own dedicated dialog if you want to flip the relay without opening Preferences. See the [Relay page]({{base}}/relay/) for the full story.

## Logging

| Field | Default | What it does |
|---|---|---|
| **Append every captured message as NDJSON to a log file** | off | Master toggle for file logging. |
| **Path** | empty | Full path to the log file. Use the **Browse…** button to pick a location with a save dialog. The file is created if it doesn't exist, never truncated, line-flushed after every message. |

See the [Logging page]({{base}}/logging/) for the JSON shape of each line.

## Appearance

| Field | Default | What it does |
|---|---|---|
| **Theme** | Dark | `System` follows your macOS appearance, `Dark` and `Light` pin the app regardless. The toolbar `T` shortcut cycles through the three at runtime. |

## Where it's stored

Settings live at:

```text
~/Library/Application Support/com.mpjhorner.MailBoxUltra/settings.json
```

It's plain JSON, schema-versioned (so older configs migrate forward when fields are added), and written atomically: each save goes to `settings.json.tmp` first, then `rename(2)` replaces the target — a crash mid-save can't leave a half-written file.

If you ever want to reset the app to factory defaults from the shell:

```sh
rm "$HOME/Library/Application Support/com.mpjhorner.MailBoxUltra/settings.json"
```

The next launch will write a fresh default file.

## Apply semantics

Clicking **Apply** validates the form, persists to disk, and calls a server restart that does the minimum work needed:

- SMTP port / bind / hostname / AUTH / max size / buffer size → SMTP listener restarts in place. Captured messages preserved.
- Relay URL / Skip TLS → relay task hot-swaps. SMTP listener untouched.
- Log file → log writer rotates to the new path or stops. Listener untouched.
- Theme → repaint only.

The toast in the bottom-right tells you what changed (`SMTP rebound to :2525 · 173 messages preserved · relay updated`).

If the new settings can't be applied (e.g. port conflict on a second instance), Apply surfaces the error inline and the old settings stay live until you fix it.
