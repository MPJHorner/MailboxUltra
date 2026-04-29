# Changelog

All notable changes are recorded here. MailBox Ultra follows [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-04-29

- HTML preview now has Desktop / iPad / Mobile size buttons so you can see how a captured email reflows at different widths. Selection is remembered across messages.
- HTML preview iframe fills the full height of the detail pane.
- Fixed: clicking a message in the list sometimes left the detail pane on the placeholder instead of showing the email.

## [0.1.0] - 2026-04-28

Initial release. Bind a port and catch every email your app tries to send. No real delivery, no setup. SMTP server, MIME parser, live web UI, JSON API, NDJSON log file, optional upstream relay.
