# scripts/

## simulate.py — fire varied messages at the running app

`scripts/simulate.py` is a stdlib-only Python 3.9+ helper that builds + sends
realistic-looking SMTP traffic at MailBox Ultra. Useful for:

- Previewing what the app looks like with a populated inbox.
- Smoke-testing a feature change end-to-end.
- Stress-testing buffer eviction (`burst -n 5000`).
- Eyeballing the HTML renderer at every device size against varied real-world
  layouts (transactional, marketing, image-heavy, dark-mode aware).

### Quick start

```sh
# 1. launch the app
open target/aarch64-apple-darwin/release/MailBoxUltra.app

# 2. fire every scenario once (skips burst by default)
./scripts/simulate.py

# 3. fire a single scenario
./scripts/simulate.py receipt

# 4. fire several, with a custom delay between them
./scripts/simulate.py welcome receipt shipping --delay 0.5

# 5. drop 200 throwaway messages (to test the ring buffer)
./scripts/simulate.py burst -n 200

# 6. run *everything* including burst
./scripts/simulate.py --all

# 7. list scenarios
./scripts/simulate.py --list
```

### Targeting a non-default app

The scripts read three env vars:

| Env var       | Default       | Notes                                     |
|---------------|---------------|-------------------------------------------|
| `SMTP_HOST`   | `127.0.0.1`   | host the app is bound to                  |
| `SMTP_PORT`   | `1025`        | the app's SMTP port (set in Preferences)  |
| `SMTP_AUTH`   | _(unset)_     | `user:pass` if AUTH is enabled            |
| `DELAY`       | `0.25`        | seconds between sends in batch mode       |

```sh
SMTP_PORT=2525 ./scripts/simulate.py
SMTP_AUTH=alice:s3cret ./scripts/simulate.py receipt
```

### Scenarios

The full list (also: `./scripts/simulate.py --list`):

| Name | What it sends |
|---|---|
| `plain` | Plain text · single recipient · short |
| `welcome` | HTML+text · branded "Welcome to ..." |
| `receipt` | HTML+text · order confirmation with line items + totals |
| `shipping` | HTML+text · "📦 your package shipped" with tracking |
| `password-reset` | HTML+text · 6-digit verification code |
| `newsletter` | HTML+text · long marketing newsletter with `List-Unsubscribe` |
| `sale` | HTML+text · loud red gradient flash-sale banner |
| `github` | Plain text · GitHub-style notification with quote depth |
| `ci-failure` | HTML+text · GitHub Actions style failure with stack trace |
| `monitor` | HTML+text · Datadog-style triggered alert |
| `survey` | HTML+text · 0-10 NPS row of clickable circles |
| `calendar` | HTML+text+`text/calendar` · meeting invite with `.ics` attachment |
| `with-pdf` | HTML+text · invoice with a real (270-byte) PDF attachment |
| `with-image` | HTML+text · two PNG attachments generated on the fly |
| `text-attach` | Plain text · `.txt` attachment |
| `many-recipients` | Plain text · 2 To + 5 Cc |
| `unicode` | Plain text · Latin / Arabic / Hebrew / CJK / emoji / math |
| `encoded-subject` | Plain text · RFC 2047 encoded-word subject |
| `long-subject` | Plain text · ~190-character subject |
| `long-body` | Plain text · ~30 KB body, hundreds of lines |
| `no-subject` | Plain text · empty `Subject:` header |
| `html-only` | HTML · single-part, no `text/plain` alternative |
| `reply-thread` | Plain text · `In-Reply-To` + `References` + nested quote |
| `dark-mode` | HTML · `prefers-color-scheme` aware |
| `marketing` | HTML+text · gradient hero + 3-up product grid |
| `burst` | Plain text · `-n` copies through a single connection |

### How attachments work

`with-pdf` carries a real, 270-byte valid PDF embedded in the script as
base64. `with-image` generates PNGs from scratch using `struct` + `zlib`
(stdlib only, no Pillow). `calendar` builds the `.ics` string inline.

That means the whole simulator works on a fresh macOS install with no `pip
install` step — Python 3 is on the box already.

### Adding a scenario

Each scenario is a one-function-per-purpose construction at the top of
`simulate.py`. Add a new `def s_my_thing(conn=None):` that builds an
`EmailMessage`, calls `send(msg, conn)`, and registers itself in the
`SCENARIOS` dict at the bottom of the file. Keep the `conn` argument so
batch runs reuse a single SMTP connection instead of reconnecting per
message.
