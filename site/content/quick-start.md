---
title: "Quick start"
description: "Capture your first email in MailBox Ultra. Run the binary, send mail, see it."
slug: quick-start
---

# Quick start

Goal: capture your first email and read every part of it. Three steps.

## 1. Run it

```sh
mailbox-ultra
```

The banner prints what it bound:

```
  ✉  MailBox Ultra v{{version}}
    SMTP    smtp://127.0.0.1:1025
    Web UI  http://127.0.0.1:8025
    Buffer  1000 messages · 25 MiB max size
```

If port 1025 or 8025 is already in use, MailBox Ultra walks forward to the next free port and prints the actual address. Want different defaults? Pass `-s 2525 -u 9000`.

## 2. Send a message

Use any SMTP client. The shortest one is [`swaks`](https://github.com/jetmore/swaks):

```sh
swaks --to dev@example.com --from app@example.com \
  --server 127.0.0.1:1025 \
  --header "Subject: Hello from MailBoxUltra" \
  --body "It works."
```

Or with `curl`:

```sh
curl --url smtp://127.0.0.1:1025 \
  --mail-from app@example.com \
  --mail-rcpt dev@example.com \
  --upload-file <(printf 'Subject: Hello\r\n\r\nIt works.\r\n')
```

## 3. Inspect it

The terminal stream picks the message up immediately:

```
  14:23:45.123  app@example.com    -> dev@example.com    Hello from MailBoxUltra    140 B
```

Open `http://127.0.0.1:8025` and you'll see the same message in the list. Click it for the full breakdown:

- **HTML** — sandboxed iframe rendering of the HTML body, if present.
- **Text** — the plain-text alternative.
- **Headers** — every header in the order they arrived, including duplicates.
- **Attachments** — one row per part, with content type, size, and a download link.
- **Source** — raw RFC 822 bytes exactly as the SMTP server received them.
- **Release** — re-send this captured message to a different SMTP server.

## Wire your real app to it

Most SMTP-aware libraries accept these standard env vars or config keys:

| Library | Setting |
|---|---|
| Laravel `MAIL_*` | `MAIL_HOST=127.0.0.1` `MAIL_PORT=1025` `MAIL_ENCRYPTION=null` |
| Rails ActionMailer | `config.action_mailer.smtp_settings = { address: '127.0.0.1', port: 1025 }` |
| Django | `EMAIL_HOST=127.0.0.1` `EMAIL_PORT=1025` `EMAIL_USE_TLS=False` |
| Node nodemailer | `nodemailer.createTransport({ host: '127.0.0.1', port: 1025, secure: false })` |
| Go gomail | `gomail.NewDialer("127.0.0.1", 1025, "", "")` |

If your app refuses to send without authentication, run with `--auth user:pass` and configure the matching credentials in the app.

## Next

- [CLI reference →]({{base}}/cli/) for every flag.
- [API reference →]({{base}}/api/) for `/api/messages` and the SSE stream.
- [Relay mode →]({{base}}/relay/) to capture *and* deliver upstream.
