use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local};
use owo_colors::OwoColorize;

use crate::message::Message;

#[derive(Clone, Debug)]
pub struct PrinterOptions {
    pub use_color: bool,
    pub json_mode: bool,
    pub verbose: bool,
    pub quiet: bool,
}

impl PrinterOptions {
    pub fn from_cli(no_cli: bool, json: bool, verbose: bool) -> Self {
        let stdout = std::io::stdout();
        let isatty = stdout.is_terminal();
        let no_color_env = std::env::var_os("NO_COLOR").is_some();
        Self {
            use_color: isatty && !no_color_env && !no_cli && !json,
            json_mode: json,
            verbose,
            quiet: no_cli && !json,
        }
    }
}

#[derive(Clone)]
pub struct Printer {
    opts: PrinterOptions,
    sink: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Printer {
    pub fn new(opts: PrinterOptions) -> Self {
        let sink: Box<dyn Write + Send> = Box::new(std::io::stdout());
        Self {
            opts,
            sink: Arc::new(Mutex::new(sink)),
        }
    }

    pub fn with_sink<W: Write + Send + 'static>(opts: PrinterOptions, sink: W) -> Self {
        Self {
            opts,
            sink: Arc::new(Mutex::new(Box::new(sink))),
        }
    }

    pub fn options(&self) -> &PrinterOptions {
        &self.opts
    }

    pub fn print_message(&self, msg: &Message) {
        if self.opts.quiet {
            return;
        }
        if self.opts.json_mode {
            if let Ok(s) = serde_json::to_string(msg) {
                self.write_line(&s);
            }
            return;
        }
        self.write_line(&self.format_line(msg));
        if self.opts.verbose {
            self.write_verbose_extras(msg);
        }
    }

    fn write_verbose_extras(&self, msg: &Message) {
        let last_idx = msg.envelope_to.len().saturating_sub(1);
        for (i, rcpt) in msg.envelope_to.iter().enumerate() {
            let is_last_to = i == last_idx;
            let no_more_after = is_last_to && msg.headers.is_empty() && msg.text.is_none();
            let connector = if no_more_after {
                "                └─"
            } else {
                "                ├─"
            };
            self.write_line(&format!("{connector} to: {rcpt}"));
        }
        let header_last = msg.headers.len().saturating_sub(1);
        for (i, (k, v)) in msg.headers.iter().enumerate() {
            let last = i == header_last && msg.text.is_none();
            let connector = if last {
                "                └─"
            } else {
                "                ├─"
            };
            self.write_line(&format!("{connector} {k}: {v}"));
        }
        if let Some(text) = &msg.text {
            let preview = body_preview(text.as_bytes(), 200);
            if !preview.is_empty() {
                self.write_line(&format!("                └─ body: {preview}"));
            }
        }
    }

    pub fn format_line(&self, msg: &Message) -> String {
        let local: DateTime<Local> = msg.received_at.with_timezone(&Local);
        let ts = local.format("%H:%M:%S%.3f").to_string();
        let from = msg
            .from
            .as_ref()
            .map(|a| a.address.clone())
            .unwrap_or_else(|| msg.envelope_from.clone());
        let from_disp = truncate(&from, 26);
        let to_first = msg
            .to
            .first()
            .map(|a| a.address.clone())
            .or_else(|| msg.envelope_to.first().cloned())
            .unwrap_or_default();
        let to_extra = if msg.envelope_to.len() > 1 {
            format!(" +{}", msg.envelope_to.len() - 1)
        } else {
            String::new()
        };
        let to_disp = truncate(&format!("{to_first}{to_extra}"), 26);
        let subject = msg
            .subject
            .as_deref()
            .unwrap_or("(no subject)")
            .replace(['\n', '\r'], " ");
        let subject_disp = truncate(&subject, 38);
        let size = humansize::format_size(msg.size as u64, humansize::BINARY);
        let attach = if msg.attachments.is_empty() {
            String::new()
        } else {
            format!(" 📎{}", msg.attachments.len())
        };

        if self.opts.use_color {
            format!(
                "  {ts}  {from:<26} {arrow} {to:<26}  {subj:<38}  {size:>10}{attach}",
                ts = ts.dimmed(),
                from = from_disp,
                arrow = "→".bright_black(),
                to = to_disp,
                subj = subject_disp.bright_white(),
                size = size.bright_black(),
                attach = attach.bright_yellow(),
            )
        } else {
            format!(
                "  {ts}  {from:<26} -> {to:<26}  {subj:<38}  {size:>10}{attach}",
                from = from_disp,
                to = to_disp,
                subj = subject_disp,
            )
        }
    }

    pub fn print_port_fallback(&self, label: &str, requested: u16, actual: u16) {
        if self.opts.json_mode || self.opts.quiet {
            return;
        }
        if self.opts.use_color {
            self.write_line(&format!(
                "  {}  {label} port {requested} in use — using {actual}",
                "!".bright_yellow(),
                label = label.bright_white(),
                requested = requested.to_string().bright_white(),
                actual = actual.to_string().bright_green(),
            ));
        } else {
            self.write_line(&format!(
                "  ! {label} port {requested} in use — using {actual}"
            ));
        }
    }

    pub fn print_update_available(&self, current: &str, latest: &str) {
        if self.opts.json_mode || self.opts.quiet {
            return;
        }
        if self.opts.use_color {
            self.write_line(&format!(
                "  {}  update available: v{current} -> v{latest}  (run `mailbox-ultra --update`)",
                "↑".bright_green(),
                current = current.bright_white(),
                latest = latest.bright_green(),
            ));
        } else {
            self.write_line(&format!(
                "  ^ update available: v{current} -> v{latest}  (run `mailbox-ultra --update`)"
            ));
        }
    }

    pub fn print_banner(
        &self,
        smtp_url: &str,
        ui_url: Option<&str>,
        buffer: usize,
        max_message: usize,
    ) {
        self.print_banner_with_relay(smtp_url, ui_url, buffer, max_message, None, false)
    }

    /// Banner with the optional relay/auth lines. `relay` is the upstream URL
    /// (already redacted of credentials by the caller); `auth_required` is
    /// true when AUTH is required on the inbound side.
    pub fn print_banner_with_relay(
        &self,
        smtp_url: &str,
        ui_url: Option<&str>,
        buffer: usize,
        max_message: usize,
        relay: Option<&str>,
        auth_required: bool,
    ) {
        if self.opts.json_mode || self.opts.quiet {
            return;
        }
        let version = env!("CARGO_PKG_VERSION");
        let max = humansize::format_size(max_message as u64, humansize::BINARY);
        if self.opts.use_color {
            self.write_line("");
            self.write_line(&format!(
                "  {} {}",
                "✉".bright_cyan(),
                format!("MailBox Ultra v{version}").bold()
            ));
            self.write_line(&format!(
                "    {}    {}",
                "SMTP".bright_black(),
                smtp_url.bright_white().underline(),
            ));
            if let Some(u) = ui_url {
                self.write_line(&format!(
                    "    {}  {}",
                    "Web UI".bright_black(),
                    u.bright_white().underline()
                ));
            }
            self.write_line(&format!(
                "    {}  {} messages · {} max size",
                "Buffer".bright_black(),
                buffer,
                max
            ));
            if auth_required {
                self.write_line(&format!(
                    "    {}    {}",
                    "Auth".bright_black(),
                    "required (PLAIN, LOGIN)".bright_yellow()
                ));
            }
            if let Some(target) = relay {
                self.write_line(&format!(
                    "    {}   -> {}",
                    "Relay".bright_black(),
                    target.bright_white().underline()
                ));
            }
            self.write_line("");
            self.write_line(&format!(
                "  {}",
                "Waiting for mail… (Ctrl+C to quit)".dimmed()
            ));
            self.write_line("");
        } else {
            self.write_line("");
            self.write_line(&format!("  MailBox Ultra v{version}"));
            self.write_line(&format!("    SMTP    {smtp_url}"));
            if let Some(u) = ui_url {
                self.write_line(&format!("    Web UI  {u}"));
            }
            self.write_line(&format!("    Buffer  {buffer} messages · {max} max size"));
            if auth_required {
                self.write_line("    Auth    required (PLAIN, LOGIN)");
            }
            if let Some(target) = relay {
                self.write_line(&format!("    Relay   -> {target}"));
            }
            self.write_line("");
            self.write_line("  Waiting for mail… (Ctrl+C to quit)");
            self.write_line("");
        }
    }

    fn write_line(&self, s: &str) {
        let mut sink = self.sink.lock().expect("printer sink poisoned");
        let _ = writeln!(sink, "{s}");
        let _ = sink.flush();
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

pub(crate) fn body_preview(body: &[u8], max: usize) -> String {
    if body.is_empty() {
        return String::new();
    }
    match std::str::from_utf8(body) {
        Ok(s) => {
            let trimmed = s.replace(['\n', '\r', '\t'], " ");
            let collapsed: String = trimmed
                .chars()
                .scan(false, |prev_space, c| {
                    let is_space = c == ' ';
                    let keep = !(is_space && *prev_space);
                    *prev_space = is_space;
                    Some(if keep { Some(c) } else { None })
                })
                .flatten()
                .collect();
            if collapsed.chars().count() > max {
                let mut out: String = collapsed.chars().take(max).collect();
                out.push('…');
                out
            } else {
                collapsed
            }
        }
        Err(_) => format!("<{} bytes binary>", body.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::sync::Arc;

    fn raw(body: &str) -> Bytes {
        Bytes::copy_from_slice(body.replace('\n', "\r\n").as_bytes())
    }

    fn msg(subject: &str) -> Message {
        crate::message::parse_message(
            raw(&format!(
                "From: \"Alice\" <alice@example.com>\nTo: bob@example.com\nSubject: {subject}\nDate: Mon, 28 Apr 2026 12:00:00 +0000\n\nbody\n"
            )),
            "alice@example.com".into(),
            vec!["bob@example.com".into()],
            "127.0.0.1:1234".into(),
            false,
        )
    }

    #[test]
    fn format_line_includes_from_to_subject_size() {
        let p = Printer::new(PrinterOptions {
            use_color: false,
            json_mode: false,
            verbose: false,
            quiet: false,
        });
        let line = p.format_line(&msg("Welcome"));
        assert!(line.contains("alice@example.com"));
        assert!(line.contains("bob@example.com"));
        assert!(line.contains("Welcome"));
        assert!(line.contains("->"));
    }

    #[test]
    fn format_line_no_subject_falls_back() {
        let p = Printer::new(PrinterOptions {
            use_color: false,
            json_mode: false,
            verbose: false,
            quiet: false,
        });
        let m = crate::message::parse_message(
            raw("From: a@x\nTo: b@x\n\nbody\n"),
            "a@x".into(),
            vec!["b@x".into()],
            "1:1".into(),
            false,
        );
        let line = p.format_line(&m);
        assert!(line.contains("(no subject)"));
    }

    #[test]
    fn format_line_attachment_marker() {
        let p = Printer::new(PrinterOptions {
            use_color: false,
            json_mode: false,
            verbose: false,
            quiet: false,
        });
        let body = "From: a@x\nTo: b@x\nSubject: hi\nMIME-Version: 1.0\nContent-Type: multipart/mixed; boundary=B\n\n--B\nContent-Type: text/plain\n\nhi\n--B\nContent-Type: text/plain; name=foo\nContent-Disposition: attachment; filename=foo.txt\n\nattachment-body\n--B--\n";
        let m = crate::message::parse_message(
            raw(body),
            "a@x".into(),
            vec!["b@x".into()],
            "1:1".into(),
            false,
        );
        assert_eq!(m.attachments.len(), 1);
        let line = p.format_line(&m);
        assert!(line.contains("📎1"));
    }

    #[test]
    fn print_message_quiet_writes_nothing() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: true,
            },
            BufWriter(buf.clone()),
        );
        p.print_message(&msg("Hi"));
        p.print_banner("smtp://x", None, 1, 1024);
        assert!(buf.lock().unwrap().is_empty());
    }

    #[test]
    fn print_message_default_writes_one_line() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        p.print_message(&msg("Hi"));
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert_eq!(out.lines().count(), 1);
        assert!(out.contains("Hi"));
    }

    #[test]
    fn verbose_mode_writes_recipients_and_headers() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: true,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        p.print_message(&msg("Hi"));
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(out.contains("to: bob@example.com"));
        assert!(out.contains("Subject"));
        assert!(out.contains("body"));
    }

    #[test]
    fn json_mode_emits_ndjson() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: true,
                verbose: false,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        p.print_message(&msg("Hi"));
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(parsed["subject"], "Hi");
    }

    #[test]
    fn banner_basic_includes_urls_and_buffer() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        p.print_banner(
            "smtp://127.0.0.1:1025",
            Some("http://127.0.0.1:8025"),
            100,
            1024,
        );
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(out.contains("MailBox Ultra"));
        assert!(out.contains("smtp://127.0.0.1:1025"));
        assert!(out.contains("http://127.0.0.1:8025"));
        assert!(out.contains("100 messages"));
    }

    #[test]
    fn banner_with_relay_and_auth_lines() {
        for use_color in [true, false] {
            let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let p = Printer::with_sink(
                PrinterOptions {
                    use_color,
                    json_mode: false,
                    verbose: false,
                    quiet: false,
                },
                BufWriter(buf.clone()),
            );
            p.print_banner_with_relay(
                "smtp://127.0.0.1:1025",
                Some("http://127.0.0.1:8025"),
                100,
                1024,
                Some("smtp://relay.example.com:25"),
                true,
            );
            let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
            assert!(out.contains("Relay"));
            assert!(out.contains("smtp://relay.example.com:25"));
            assert!(out.contains("Auth"));
        }
    }

    #[test]
    fn banner_quiet_and_json_emit_nothing() {
        for opts in [
            PrinterOptions {
                use_color: false,
                json_mode: true,
                verbose: false,
                quiet: false,
            },
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: true,
            },
        ] {
            let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let p = Printer::with_sink(opts, BufWriter(buf.clone()));
            p.print_banner_with_relay("smtp://x", None, 1, 1024, Some("smtp://relay"), true);
            assert!(buf.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn port_fallback_notice_includes_both_ports() {
        for use_color in [true, false] {
            let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let p = Printer::with_sink(
                PrinterOptions {
                    use_color,
                    json_mode: false,
                    verbose: false,
                    quiet: false,
                },
                BufWriter(buf.clone()),
            );
            p.print_port_fallback("SMTP", 1025, 1026);
            let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
            assert!(out.contains("1025"));
            assert!(out.contains("1026"));
        }
    }

    #[test]
    fn port_fallback_quiet_writes_nothing() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let p = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: true,
            },
            BufWriter(buf.clone()),
        );
        p.print_port_fallback("SMTP", 1, 2);
        assert!(buf.lock().unwrap().is_empty());
    }

    #[test]
    fn update_available_notice_branches() {
        for opts in [
            PrinterOptions {
                use_color: true,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
        ] {
            let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let p = Printer::with_sink(opts, BufWriter(buf.clone()));
            p.print_update_available("0.1.0", "0.2.0");
            let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
            assert!(out.contains("update available"));
            assert!(out.contains("0.2.0"));
        }
    }

    #[test]
    fn options_from_cli_quiet_when_no_cli_and_not_json() {
        let opts = PrinterOptions::from_cli(true, false, false);
        assert!(opts.quiet);
        let opts = PrinterOptions::from_cli(true, true, false);
        assert!(!opts.quiet);
        assert!(opts.json_mode);
    }

    #[test]
    fn body_preview_handles_text_binary_long_empty() {
        assert_eq!(body_preview(b"", 10), "");
        assert_eq!(body_preview(b"hi\nthere", 10), "hi there");
        let long = vec![b'a'; 500];
        let p = body_preview(&long, 10);
        assert!(p.ends_with('…'));
        let bin = vec![0xff, 0xfe, 0xfd];
        assert_eq!(body_preview(&bin, 10), "<3 bytes binary>");
    }

    #[test]
    fn truncate_works() {
        assert_eq!(truncate("abc", 10), "abc");
        let t = truncate("abcdefghijklmnop", 7);
        assert_eq!(t.chars().count(), 7);
        assert!(t.ends_with('…'));
    }

    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
