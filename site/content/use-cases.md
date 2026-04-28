---
title: "Use cases"
description: "Transactional mail, password resets, SDK inspection, AI pairing — concrete recipes for MailBox Ultra."
slug: use-cases
---

# Use cases

A handful of recipes for things you'll do over and over.

## Stop polluting test inboxes

Most apps need real-looking SMTP credentials in dev to function. Pointing them at MailBox Ultra means *every* outgoing email lands in your local UI instead of `dev@yourcompany.com` (and worse, occasionally a real customer when someone forgets to suppress sends in staging).

```env
MAIL_HOST=127.0.0.1
MAIL_PORT=1025
MAIL_FROM_ADDRESS=app@local.test
MAIL_ENCRYPTION=null
```

## Transactional mail QA

Render a password-reset email, an invoice, an order-shipped notification. Compare HTML and text alternatives, scrutinise headers (`Reply-To`, `Auto-Submitted`, `List-Unsubscribe-Post`), download attachments, view the raw RFC 822 source for issues a parser would smooth over.

## Reverse-engineer a sender library

Curious what your queue worker actually emits? Point it at MailBox Ultra and inspect:

- Does the From header match the envelope sender?
- Is the message multipart/alternative or just one body?
- Are attachments attached or referenced (`Content-Disposition`)?
- What `Content-Transfer-Encoding` does it pick?

The Source tab is the source of truth. Everything else is parsed for your convenience.

## Capture-and-deliver staging

Want to read every email a staging environment sends *and* still send them out for real? Run with `--relay` pointing at the real upstream MTA. MailBox Ultra captures, then relays. If the upstream is down, the message stays in the buffer until you hit Release.

```sh
mailbox-ultra --relay smtp://relay.example.com:25
```

## AI-assistant pairing

Hand an LLM a tail-able feed of mail your app is sending while you fix something else.

```sh
mailbox-ultra --relay smtp://relay.example.com:25 \
  --log-file /tmp/mail.ndjson
```

Then in the assistant's tool config, give it `tail -f /tmp/mail.ndjson | jq` so it can react to subject lines, attachment counts, or specific recipients in real time.

## Replay a flow

Capture a signup email once. Hit Release, point at your dev SMTP, and the same captured message goes out again. Useful for clicking confirmation links a hundred times without burning through verification quotas.

## Onboarding test runs

In CI:

```sh
mailbox-ultra --no-cli --json --no-update-check > /tmp/mbu.ndjson &
PID=$!
sleep 1
# … run tests that send mail …
# Assert the captured set matches expectations.
jq -s 'length' /tmp/mbu.ndjson
kill $PID
```

The JSON shape is stable across patch releases. Major version changes will be called out in the changelog.

## Devcontainer / docker-compose

```yaml
services:
  mailbox-ultra:
    image: rust:1.85-slim
    command: >
      bash -c "cargo install --git https://github.com/MPJHorner/MailboxUltra && mailbox-ultra --bind 0.0.0.0"
    ports:
      - "1025:1025"
      - "8025:8025"
```

A pre-built image is on the roadmap but not yet shipped.
