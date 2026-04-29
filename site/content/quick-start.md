---
title: "Quick start"
description: "Open the MailBox Ultra app, send your first email, walk every detail tab. Three steps."
slug: quick-start
---

# Quick start

Goal: capture your first email and read every part of it. Three steps.

## 1. Open the app

Double-click **MailBox Ultra** in `/Applications` (or hit Spotlight, type `mailbox`, press Return). The window appears, the SMTP server starts, and the toolbar shows the URL it bound:

```text
SMTP  smtp://127.0.0.1:1025
```

If port 1025 is busy, MailBox Ultra walks forward to the next free port and the toolbar reflects whatever it actually got. Click the SMTP pill to copy it.

Need different defaults? Hit `⌘,` to open Preferences and change the port, bind address, hostname, or anything else.

## 2. Send a message

Use any SMTP client. The shortest one is [`swaks`](https://github.com/jetmore/swaks):

```sh
swaks --to dev@example.com --from app@example.com \
  --server 127.0.0.1:1025 \
  --header "Subject: Hello from MailBox Ultra" \
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

The new message lands in the inbox immediately. Click the row in the sidebar to open it; the detail pane on the right is six tabs deep:

- **HTML** — captured HTML body painted by an embedded `WKWebView`. Switch between **Mobile (390)**, **iPad (834)**, and **Desktop (1280)** to flex `@media` queries with the matching User-Agent. The "Open in browser" button hands the HTML off to your default browser if you want a full-page view.
- **Text** — the plain-text alternative.
- **Headers** — every header in the order they arrived, including duplicates.
- **Attachments** — one row per part, with content type, size, and a Save… button.
- **Source** — raw RFC 822 bytes exactly as the SMTP server received them.
- **Release** — re-send this captured message to a different SMTP server.

Number keys `1` – `6` switch between the tabs. `j` / `k` (or `↓` / `↑`) move between captured messages.

## Wire your real app to it

Most SMTP-aware libraries accept these standard env vars or config keys:

| Library | Setting |
|---|---|
| Laravel `MAIL_*` | `MAIL_HOST=127.0.0.1` `MAIL_PORT=1025` `MAIL_ENCRYPTION=null` |
| Rails ActionMailer | `config.action_mailer.smtp_settings = { address: '127.0.0.1', port: 1025 }` |
| Django | `EMAIL_HOST=127.0.0.1` `EMAIL_PORT=1025` `EMAIL_USE_TLS=False` |
| Node nodemailer | `nodemailer.createTransport({ host: '127.0.0.1', port: 1025, secure: false })` |
| Go gomail | `gomail.NewDialer("127.0.0.1", 1025, "", "")` |

If your app refuses to send without authentication, open Preferences (`⌘,`), tick **Require AUTH** under SMTP, set a user and password, click **Apply**. Match those credentials in the app.

## Useful keys

| Keys | Action |
|---|---|
| `⌘,` | Open Preferences |
| `?` | Show every keyboard shortcut |
| `⇧⌘X` | Clear the inbox |
| `⌘Q` | Quit |

The full cheat-sheet lives behind `?`.

## Next

- [Preferences reference →]({{base}}/configuration/) — every field in the Preferences window.
- [Relay mode →]({{base}}/relay/) — capture *and* deliver upstream.
- [SMTP server details →]({{base}}/smtp/) — supported verbs, AUTH, size limits.
