---
title: "SMTP server"
description: "Supported SMTP commands, AUTH mechanisms, size limits, and behavioural notes."
slug: smtp
---

# SMTP server

MailBox Ultra implements just enough of [RFC 5321](https://datatracker.ietf.org/doc/html/rfc5321) to play nice with every mainstream sender library and CLI we've tested. The list below is exhaustive — anything not on it returns `500 5.5.2`.

## Supported verbs

| Verb | Behaviour |
|---|---|
| `HELO` | Records that the client greeted; replies `250`. |
| `EHLO` | Returns the multi-line capability list (see below). |
| `MAIL FROM:<addr>` | Stores the envelope sender. Empty `<>` is allowed (RFC 5321 null sender). |
| `RCPT TO:<addr>` | Stores one envelope recipient. Repeat for each recipient. |
| `DATA` | Reads body until `\r\n.\r\n`, dot-unstuffs, parses MIME, stores. |
| `RSET` | Clears the envelope. |
| `NOOP` | Returns `250`. |
| `QUIT` | Returns `221` and closes. |
| `HELP` | Lists the supported verbs. |
| `VRFY` | Returns `252` (per RFC, "cannot verify but will accept"). |
| `AUTH PLAIN \| LOGIN` | See [authentication](#authentication). |

`STARTTLS` is reserved for a future release. There is no plan to implement legacy `SOML`, `SAML`, `EXPN`, or `TURN`.

## EHLO capabilities

```text
250-MailBoxUltra hello
250-PIPELINING
250-8BITMIME
250-SMTPUTF8
250-SIZE 26214400
250-AUTH PLAIN LOGIN     (only when --auth is set)
250 HELP
```

`SIZE` mirrors `--max-message-size` (default 25 MiB).

## Authentication

Set `--auth user:pass` to require authentication. Without the flag, no AUTH is advertised and the server accepts anyone. With the flag, attempting `MAIL FROM` before authenticating returns:

```text
530 5.7.0 authentication required
```

Both `AUTH PLAIN` and `AUTH LOGIN` are supported, in initial-response and prompt-response forms.

### AUTH PLAIN

Inline form:

```text
C: AUTH PLAIN AGFsaWNlAHMzY3JldA==
S: 235 2.7.0 authentication successful
```

Or two-step:

```text
C: AUTH PLAIN
S: 334
C: AGFsaWNlAHMzY3JldA==
S: 235 2.7.0 authentication successful
```

Wrong credentials get `535 5.7.8`.

### AUTH LOGIN

```text
C: AUTH LOGIN
S: 334 VXNlcm5hbWU6           (Username:)
C: YWxpY2U=                   (alice)
S: 334 UGFzc3dvcmQ6           (Password:)
C: czNjcmV0                   (s3cret)
S: 235 2.7.0 authentication successful
```

The server is permissive about case and whitespace. Initial-response form (username on the same line as `AUTH LOGIN`) is also accepted, as that's what some older clients send.

## Size limits

`--max-message-size` rejects oversize bodies with `552 5.3.4` after consuming the data so the client receives a clean response. The captured envelope is reset on rejection — RSET semantics, automatically.

## Capture vs. delivery

Plain capture: nothing is delivered, the message lands in the in-memory buffer and the [JSON+SSE API]({{base}}/api/), and the SMTP transaction returns `250 2.0.0 message accepted`.

With `--relay smtp://upstream`, the same flow runs *and* the relay task hands the message to the upstream MTA. If the relay fails, the captured message is still in the buffer; the failure is logged at `warn` level. See [relay mode]({{base}}/relay/) for details.

## Hostname

`--hostname NAME` controls what the server announces in the `220` greeting and the `250 NAME hello` response. Default is `MailBoxUltra`. Some sender libraries pin to a specific hostname for testing; this is the knob.

## Rejected verbs

Anything outside the supported list returns:

```text
500 5.5.2 unrecognised command: FOO
```

Empty lines return `500 5.5.2 syntax error: empty command`. Malformed `MAIL`/`RCPT` parameters return `501 5.5.4 syntax: ...`.

## Wire-level testing

Want to poke the protocol by hand? Use `nc`:

```sh
nc -C 127.0.0.1 1025
```

Type `EHLO me`, `MAIL FROM:<a@x>`, `RCPT TO:<b@x>`, `DATA`, the body, `.`, `QUIT`. The web UI updates in real time.
