---
title: "Use cases"
description: "Transactional mail, library inspection, capture-and-relay, AI pairing — concrete recipes for MailBox Ultra."
slug: use-cases
---

# Use cases

A handful of recipes for things you'll do over and over.

## Stop polluting test inboxes

Most apps need real-looking SMTP credentials in dev to function. Pointing them at MailBox Ultra means *every* outgoing email lands in the local app instead of `dev@yourcompany.com` (and worse, occasionally a real customer when someone forgets to suppress sends in staging).

```env
MAIL_HOST=127.0.0.1
MAIL_PORT=1025
MAIL_FROM_ADDRESS=app@local.test
MAIL_ENCRYPTION=null
```

## Transactional mail QA

Render a password-reset email, an invoice, an order-shipped notification. Compare HTML and text alternatives, scrutinise headers (`Reply-To`, `Auto-Submitted`, `List-Unsubscribe-Post`), download attachments from the **Attachments** tab, view the raw RFC 822 source for issues a parser would smooth over.

The HTML tab paints captured email through `WKWebView`, the same engine Mail.app uses, so what you see is faithful to how the real client renders it.

## Responsive HTML preview

The HTML tab has three width buttons:

- **Mobile (390)** — iOS Safari User-Agent, 390 px viewport. `@media (max-width: 600px)` rules fire.
- **iPad (834)** — iPadOS Safari User-Agent, 834 px viewport.
- **Desktop (1280)** — desktop Safari User-Agent, 1280 px viewport.

Switching widths swaps both the viewport *and* the User-Agent so server-side responsive frameworks (Mailchimp, MJML, hand-rolled `@media`) trip the same paths the real client would.

## Reverse-engineer a sender library

Curious what your queue worker actually emits? Point it at MailBox Ultra and inspect:

- Does the `From` header match the envelope sender?
- Is the message `multipart/alternative` or just one body?
- Are attachments attached or referenced (`Content-Disposition`)?
- What `Content-Transfer-Encoding` does it pick?

The **Source** tab is the source of truth. Everything else is parsed for your convenience.

## Capture-and-deliver staging

Want to read every email a staging environment sends *and* still send them out for real? Open Preferences (`⌘,`), tick **Forward each captured message upstream** under Relay, paste an `smtp://` or `smtps://` URL, click **Apply**. Capture happens first, relay happens after — if the upstream is down, the message stays in the inbox and you can resend it manually from the **Release** tab. See the [Relay page]({{base}}/relay/) for the full story.

## AI-assistant pairing

Hand a coding agent a tail-able feed of mail your app is sending while you keep working in the GUI:

1. Open Preferences with `⌘,`.
2. Tick **Append every captured message as NDJSON to a log file** under Logging.
3. Pick a path, e.g. `/tmp/mail.ndjson`.
4. Click **Apply**.

Then in the assistant's tool config, give it `tail -f /tmp/mail.ndjson | jq` so it can react to subject lines, attachment counts, or specific recipients in real time. Add **Forward each captured message upstream** under Relay if you also want the messages delivered for real.

The [Logging page]({{base}}/logging/) documents the JSON shape on each line.

## Replay a flow

Capture a signup email once. Open the **Release** tab on that message, point at your dev SMTP, and the same captured message goes out again. Useful for clicking confirmation links a hundred times without burning through verification quotas.

## Onboarding test runs

Run the app, send mail through it from your test suite, then either inspect by hand or point a script at the NDJSON log file. The [Logging page]({{base}}/logging/) shows the JSON shape — `id`, `received_at`, `envelope_to`, `subject`, `headers`, etc., one object per line. The shape is stable across patch releases and any breaking change is called out in the [changelog]({{base}}/changelog/).

For a totally clean inbox between runs, hit `⇧⌘X` (Clear inbox) or just relaunch the app.

## Devcontainer / docker host

If your dev container runs on the same Mac, set the bind address to `0.0.0.0`:

1. `⌘,` to open Preferences.
2. Set **Bind address** to `0.0.0.0`.
3. **Apply**.

The container then reaches the SMTP listener at `host.docker.internal:1025` (or whatever `host.docker.internal` resolves to on your host).
