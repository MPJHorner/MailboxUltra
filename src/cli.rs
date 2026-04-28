use std::net::IpAddr;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "mailbox-ultra",
    version,
    about = "Local SMTP fake inbox. Catch every email your app sends without ever delivering one.",
    long_about = "MailBox Ultra runs a real SMTP server on a port you choose, parses every message that lands on it, and shows the result in a live web UI. Nothing is delivered. Optional --relay turns it into a transparent relay that captures the message and then hands it off to a real upstream MTA. Optional --log-file appends every captured message as one JSON object per line so you can tail it from a script or an AI assistant."
)]
pub struct Cli {
    /// Port the SMTP server listens on. Point your sender here.
    #[arg(short = 's', long = "smtp-port", default_value_t = 1025)]
    pub smtp_port: u16,

    /// Port the web UI listens on.
    #[arg(short = 'u', long = "ui-port", default_value_t = 8025)]
    pub ui_port: u16,

    /// Bind address for both servers.
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,

    /// Hostname announced in the SMTP banner and EHLO response.
    #[arg(long, default_value = "MailBoxUltra")]
    pub hostname: String,

    /// Maximum SMTP message size in bytes. Larger DATA payloads are rejected.
    #[arg(long, default_value_t = 25 * 1024 * 1024)]
    pub max_message_size: usize,

    /// Number of messages to keep in memory.
    #[arg(long, default_value_t = 1000)]
    pub buffer_size: usize,

    /// Disable the web UI server entirely.
    #[arg(long)]
    pub no_ui: bool,

    /// Disable the colored CLI output.
    #[arg(long)]
    pub no_cli: bool,

    /// Emit each captured message as JSON (NDJSON) to stdout.
    #[arg(long, conflicts_with = "no_cli")]
    pub json: bool,

    /// Open the web UI in your browser on startup.
    #[arg(long)]
    pub open: bool,

    /// Verbose CLI output: prints recipients, headers, and a body preview for each message.
    #[arg(short, long)]
    pub verbose: bool,

    /// Download the latest release from GitHub and replace this binary, then exit.
    #[arg(long)]
    pub update: bool,

    /// Skip the startup check that asks GitHub if a newer release is available.
    #[arg(long)]
    pub no_update_check: bool,

    /// Require AUTH PLAIN / AUTH LOGIN with the given USER:PASS credentials.
    /// Without this flag the server accepts any client. Useful when your sender
    /// will not connect unless authentication is offered.
    #[arg(long, value_name = "USER:PASS")]
    pub auth: Option<String>,

    /// Relay each captured message to an upstream SMTP URL after capturing it.
    /// Format: smtp://host:port or smtps://host:port. Path/query are ignored.
    /// Userinfo (smtp://user:pass@host) is used as AUTH PLAIN credentials.
    #[arg(long, value_name = "URL")]
    pub relay: Option<String>,

    /// Skip TLS certificate verification when relaying upstream (dev/staging only).
    #[arg(long)]
    pub relay_insecure: bool,

    /// Append every captured message to FILE as one JSON object per line (NDJSON).
    /// The file is created if missing and never truncated. Useful when pairing
    /// with --relay to give an AI assistant or other tool a tail-able feed
    /// of messages as they arrive.
    #[arg(long, value_name = "FILE")]
    pub log_file: Option<String>,
}

impl Cli {
    pub fn validate(&self) -> Result<(), String> {
        self.bind
            .parse::<IpAddr>()
            .map_err(|_| format!("invalid bind address: {}", self.bind))?;
        // Port 0 means "ephemeral, OS-assigned" so two zeroes resolve to two
        // different ports -- only reject explicit clashes on a real port.
        if !self.no_ui && self.smtp_port != 0 && self.smtp_port == self.ui_port {
            return Err(format!(
                "SMTP port and UI port cannot both be {} -- pass --ui-port or --no-ui",
                self.smtp_port
            ));
        }
        if self.max_message_size == 0 {
            return Err("--max-message-size must be > 0".into());
        }
        if self.buffer_size == 0 {
            return Err("--buffer-size must be > 0".into());
        }
        if self.hostname.trim().is_empty() {
            return Err("--hostname must not be empty".into());
        }
        if let Some(creds) = &self.auth {
            if !creds.contains(':') {
                return Err("--auth must be in USER:PASS form".into());
            }
            let (u, _) = creds.split_once(':').unwrap();
            if u.is_empty() {
                return Err("--auth user must not be empty".into());
            }
        }
        if let Some(url) = &self.relay {
            let parsed =
                url::Url::parse(url).map_err(|e| format!("invalid --relay URL '{url}': {e}"))?;
            match parsed.scheme() {
                "smtp" | "smtps" => {}
                other => return Err(format!("--relay URL must use smtp or smtps, got '{other}'")),
            }
            if parsed.host().is_none() {
                return Err(format!("--relay URL '{url}' is missing a host"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        let mut full = vec!["mailbox-ultra"];
        full.extend_from_slice(args);
        Cli::parse_from(full)
    }

    #[test]
    fn defaults_match_documented_values() {
        let cli = parse(&[]);
        assert_eq!(cli.smtp_port, 1025);
        assert_eq!(cli.ui_port, 8025);
        assert_eq!(cli.bind, "127.0.0.1");
        assert_eq!(cli.hostname, "MailBoxUltra");
        assert_eq!(cli.max_message_size, 25 * 1024 * 1024);
        assert_eq!(cli.buffer_size, 1000);
        assert!(!cli.no_ui);
        assert!(!cli.no_cli);
        assert!(!cli.json);
        assert!(!cli.open);
        assert!(!cli.verbose);
        cli.validate().unwrap();
    }

    #[test]
    fn parses_short_and_long_flags() {
        let cli = parse(&["-s", "2525", "-u", "8888", "-v"]);
        assert_eq!(cli.smtp_port, 2525);
        assert_eq!(cli.ui_port, 8888);
        assert!(cli.verbose);
    }

    #[test]
    fn parses_long_flags() {
        let cli = parse(&[
            "--smtp-port",
            "2025",
            "--ui-port",
            "8025",
            "--bind",
            "0.0.0.0",
            "--max-message-size",
            "2048",
            "--buffer-size",
            "10",
            "--no-ui",
        ]);
        assert_eq!(cli.smtp_port, 2025);
        assert_eq!(cli.ui_port, 8025);
        assert_eq!(cli.bind, "0.0.0.0");
        assert_eq!(cli.max_message_size, 2048);
        assert_eq!(cli.buffer_size, 10);
        assert!(cli.no_ui);
        cli.validate().unwrap();
    }

    #[test]
    fn validate_rejects_same_port_when_ui_enabled() {
        let cli = parse(&["-s", "1025", "-u", "1025"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("1025"));
    }

    #[test]
    fn validate_allows_same_port_when_ui_disabled() {
        let cli = parse(&["-s", "1025", "-u", "1025", "--no-ui"]);
        cli.validate().unwrap();
    }

    #[test]
    fn validate_rejects_invalid_bind() {
        let cli = parse(&["--bind", "not-an-ip"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("invalid"));
    }

    #[test]
    fn validate_rejects_zero_max_message() {
        let cli = parse(&["--max-message-size", "0"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("max-message-size"));
    }

    #[test]
    fn validate_rejects_zero_buffer() {
        let cli = parse(&["--buffer-size", "0"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("buffer-size"));
    }

    #[test]
    fn validate_rejects_empty_hostname() {
        let cli = parse(&["--hostname", "   "]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("hostname"));
    }

    #[test]
    fn json_and_no_cli_conflict() {
        let res = Cli::try_parse_from(["mailbox-ultra", "--json", "--no-cli"]);
        assert!(res.is_err());
    }

    #[test]
    fn cli_is_clone() {
        let cli = parse(&[]);
        let _cloned = cli.clone();
    }

    #[test]
    fn auth_requires_colon() {
        let cli = parse(&["--auth", "no-colon"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("USER:PASS"));
    }

    #[test]
    fn auth_user_must_be_non_empty() {
        let cli = parse(&["--auth", ":secret"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("user"));
    }

    #[test]
    fn auth_accepts_well_formed_credentials() {
        let cli = parse(&["--auth", "alice:s3cret"]);
        cli.validate().unwrap();
    }

    #[test]
    fn relay_accepts_smtp_and_smtps() {
        for url in ["smtp://relay.example.com:25", "smtps://r.example.com:465"] {
            let cli = parse(&["--relay", url]);
            cli.validate().expect(url);
        }
    }

    #[test]
    fn relay_rejects_invalid_url() {
        let cli = parse(&["--relay", "not-a-url"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("invalid --relay URL"));
    }

    #[test]
    fn relay_rejects_non_smtp_scheme() {
        let cli = parse(&["--relay", "http://example.com"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("must use smtp or smtps"));
    }

    #[test]
    fn relay_rejects_missing_host() {
        let cli = parse(&["--relay", "smtp:///foo"]);
        let err = cli.validate().unwrap_err();
        assert!(err.contains("missing a host"));
    }
}
