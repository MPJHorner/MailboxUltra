---
title: "Contributing"
description: "How to contribute to MailBox Ultra: conventions, coverage policy, release flow."
slug: contributing
---

# Contributing

Thanks for poking at this. Issues and pull requests are welcome.

## Local checks

```sh
make check         # fmt + clippy + tests
make coverage      # llvm-cov summary, matches CI exclusions
make coverage-html # per-line HTML report at target/llvm-cov/html/index.html
make smoke         # end-to-end smoke against a fresh release binary
```

`make check` is the same gate CI runs. PRs that fail it get sent back automatically.

## Test policy

Every feature ships with a test. The project tracks coverage on the testable surface (everything outside `src/main.rs`, `src/assets.rs`, `src/update.rs`, `src/entrypoint.rs` — those are excluded with reasons documented in [CLAUDE.md](https://github.com/MPJHorner/MailboxUltra/blob/main/CLAUDE.md)).

When adding code, prefer:

- A unit test inside the same module (`mod tests` block at the bottom of the file) for pure logic.
- An integration test under `tests/` for anything that drives a real socket or the HTTP UI.
- The existing helper `app::start` for spinning up the full pipeline in tests; it accepts `-s 0 -u 0` for ephemeral ports and returns handles you can `.shutdown()`.

## Style

- No em dashes in user-facing text.
- No AI-slop adjectives (blazing-fast, beautifully-designed, etc).
- README leads with a single short pitch and links to this docs site.
- Inline comments explain *why*, not *what*. The reader can read the code.

## Versioning and release flow

[Semantic Versioning](https://semver.org/), conventional commits.

When you commit a user-visible change, do all four in the same commit:

1. Bump `version` in `Cargo.toml`. `feat:` → minor, `fix:`/no prefix → patch, `[major]` or `BREAKING CHANGE:` → major.
2. Add a top entry to `CHANGELOG.md`. Keep it terse and user-facing.
3. Run `make check`.
4. After merging to `main`, push the matching tag:

```sh
git tag "v$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
git push --tags
```

The release workflow handles the rest: builds for macOS (Intel + Apple Silicon), Linux (x86_64 + aarch64), and Windows (x86_64); publishes a GitHub release; uploads the archives + sha256 sums; redeploys the docs site on every push to `main` and every `v*` tag.

## Where things live

- `src/smtp.rs` — listener, command parser, session state machine.
- `src/message.rs` — captured message type, MIME parsing wrapper.
- `src/store.rs` — bounded ring buffer + broadcast channel.
- `src/relay.rs` — optional upstream relay.
- `src/ui.rs` — JSON API, SSE stream, attachment downloads, relay endpoints.
- `src/output.rs` — terminal printer + banner.
- `ui/` — the embedded vanilla-JS web UI (no build step).
- `tests/` — integration tests; `lettre` for SMTP, `reqwest` + `eventsource-client` for HTTP.
- `site/` — this docs site (handwritten static-site builder, deployed by the `site` workflow).

## Code of conduct

Be kind. We have a zero-tolerance policy for harassment. Disagreements about technical decisions are welcome; ad-hominem attacks are not.
