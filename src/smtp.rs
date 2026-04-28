//! Tokio-based SMTP server. Parses just enough of RFC 5321 to capture every
//! message a developer's app might try to send, then hands the parsed result
//! to the [`MessageStore`].
//!
//! Supported verbs: HELO, EHLO, MAIL, RCPT, DATA, RSET, NOOP, QUIT, HELP,
//! VRFY, AUTH (PLAIN, LOGIN). STARTTLS is reserved for a future release.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::message::parse_message;
use crate::store::MessageStore;

/// Runtime-tweakable SMTP options.
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub hostname: String,
    pub max_message_size: usize,
    /// `(user, pass)`. When `Some`, AUTH PLAIN / AUTH LOGIN are advertised in
    /// EHLO and required before MAIL FROM is accepted.
    pub auth: Option<(String, String)>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            hostname: "MailBoxUltra".into(),
            max_message_size: 25 * 1024 * 1024,
            auth: None,
        }
    }
}

/// Bind an SMTP listener and accept connections forever, spawning one task
/// per session.
pub async fn serve(
    listener: TcpListener,
    store: Arc<MessageStore>,
    config: SmtpConfig,
) -> Result<()> {
    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .context("accepting SMTP connection")?;
        let store = store.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_session(stream, peer, store, cfg).await {
                tracing::debug!(peer = %peer, error = %e, "SMTP session ended with error");
            }
        });
    }
}

pub async fn handle_session(
    stream: TcpStream,
    peer: SocketAddr,
    store: Arc<MessageStore>,
    cfg: SmtpConfig,
) -> Result<()> {
    stream.set_nodelay(true).ok();
    let (read, write) = stream.into_split();
    run_session(BufReader::new(read), write, peer, store, cfg).await
}

/// Generic session driver, parameterised on the underlying I/O so unit tests
/// can drive it via in-memory pipes without a real socket.
pub async fn run_session<R, W>(
    mut reader: BufReader<R>,
    mut writer: W,
    peer: SocketAddr,
    store: Arc<MessageStore>,
    cfg: SmtpConfig,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut state = Session::new(cfg.clone(), peer);

    write_line(
        &mut writer,
        &format!("220 {} ESMTP MailBoxUltra ready", cfg.hostname),
    )
    .await?;

    loop {
        let line = match read_line(&mut reader, 4096).await? {
            Some(l) => l,
            None => return Ok(()),
        };
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            write_line(&mut writer, "500 5.5.2 syntax error: empty command").await?;
            continue;
        }
        let (verb, rest) = split_verb(trimmed);
        let upper = verb.to_ascii_uppercase();
        match upper.as_str() {
            "HELO" => {
                state.helo = true;
                write_line(&mut writer, &format!("250 {} hello", state.cfg.hostname)).await?;
            }
            "EHLO" => {
                state.helo = true;
                let lines = state.ehlo_lines();
                for l in lines {
                    write_line(&mut writer, &l).await?;
                }
            }
            "MAIL" => {
                let reply = state.handle_mail(rest);
                write_line(&mut writer, &reply).await?;
            }
            "RCPT" => {
                let reply = state.handle_rcpt(rest);
                write_line(&mut writer, &reply).await?;
            }
            "DATA" => {
                if state.envelope.from.is_none() || state.envelope.rcpts.is_empty() {
                    write_line(&mut writer, "503 5.5.1 need MAIL FROM and RCPT TO first").await?;
                    continue;
                }
                write_line(&mut writer, "354 end with <CRLF>.<CRLF>").await?;
                match read_data_body(&mut reader, state.cfg.max_message_size).await? {
                    DataOutcome::Done(body) => {
                        let env = std::mem::take(&mut state.envelope);
                        let msg = parse_message(
                            body,
                            env.from.unwrap_or_default(),
                            env.rcpts,
                            peer.to_string(),
                            state.auth.authenticated,
                        );
                        store.push(msg);
                        write_line(&mut writer, "250 2.0.0 message accepted").await?;
                    }
                    DataOutcome::TooLarge => {
                        state.envelope = Envelope::default();
                        write_line(
                            &mut writer,
                            &format!(
                                "552 5.3.4 message exceeds size limit of {} bytes",
                                state.cfg.max_message_size
                            ),
                        )
                        .await?;
                    }
                    DataOutcome::Closed => return Ok(()),
                }
            }
            "RSET" => {
                state.envelope = Envelope::default();
                write_line(&mut writer, "250 2.0.0 OK").await?;
            }
            "NOOP" => {
                write_line(&mut writer, "250 2.0.0 OK").await?;
            }
            "QUIT" => {
                write_line(
                    &mut writer,
                    &format!("221 2.0.0 {} closing connection", state.cfg.hostname),
                )
                .await?;
                return Ok(());
            }
            "VRFY" => {
                write_line(
                    &mut writer,
                    "252 2.5.2 cannot VRFY user, but will accept message",
                )
                .await?;
            }
            "HELP" => {
                write_line(
                    &mut writer,
                    "214 2.0.0 Commands: HELO EHLO MAIL RCPT DATA RSET NOOP QUIT AUTH HELP VRFY",
                )
                .await?;
            }
            "AUTH" => {
                handle_auth(&mut state, &mut reader, &mut writer, rest).await?;
            }
            other => {
                write_line(
                    &mut writer,
                    &format!("500 5.5.2 unrecognised command: {other}"),
                )
                .await?;
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Envelope {
    from: Option<String>,
    rcpts: Vec<String>,
}

#[derive(Debug, Default)]
struct AuthState {
    authenticated: bool,
}

#[derive(Debug)]
struct Session {
    cfg: SmtpConfig,
    #[allow(dead_code)]
    peer: SocketAddr,
    helo: bool,
    envelope: Envelope,
    auth: AuthState,
}

impl Session {
    fn new(cfg: SmtpConfig, peer: SocketAddr) -> Self {
        Self {
            cfg,
            peer,
            helo: false,
            envelope: Envelope::default(),
            auth: AuthState::default(),
        }
    }

    fn auth_required(&self) -> bool {
        self.cfg.auth.is_some() && !self.auth.authenticated
    }

    fn ehlo_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("250-{} hello", self.cfg.hostname),
            "250-PIPELINING".into(),
            "250-8BITMIME".into(),
            "250-SMTPUTF8".into(),
            format!("250-SIZE {}", self.cfg.max_message_size),
        ];
        if self.cfg.auth.is_some() {
            lines.push("250-AUTH PLAIN LOGIN".into());
        }
        lines.push("250 HELP".into());
        lines
    }

    fn handle_mail(&mut self, rest: &str) -> String {
        if !self.helo {
            return "503 5.5.1 send HELO/EHLO first".into();
        }
        if self.auth_required() {
            return "530 5.7.0 authentication required".into();
        }
        let lower = rest.to_ascii_lowercase();
        let prefix = "from:";
        let Some(_) = lower.strip_prefix(prefix) else {
            return "501 5.5.4 syntax: MAIL FROM:<address>".into();
        };
        let addr_part = &rest[prefix.len()..];
        let addr = extract_address(addr_part).unwrap_or_default();
        self.envelope.from = Some(addr);
        self.envelope.rcpts.clear();
        "250 2.1.0 sender ok".into()
    }

    fn handle_rcpt(&mut self, rest: &str) -> String {
        if self.envelope.from.is_none() {
            return "503 5.5.1 need MAIL FROM first".into();
        }
        let lower = rest.to_ascii_lowercase();
        let prefix = "to:";
        if lower.strip_prefix(prefix).is_none() {
            return "501 5.5.4 syntax: RCPT TO:<address>".into();
        }
        let addr_part = &rest[prefix.len()..];
        let Some(addr) = extract_address(addr_part) else {
            return "501 5.5.4 invalid recipient address".into();
        };
        if addr.is_empty() {
            return "501 5.5.4 invalid recipient address".into();
        }
        self.envelope.rcpts.push(addr);
        "250 2.1.5 recipient ok".into()
    }
}

async fn handle_auth<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    state: &mut Session,
    reader: &mut BufReader<R>,
    writer: &mut W,
    rest: &str,
) -> Result<()> {
    let Some((user, pass)) = state.cfg.auth.clone() else {
        write_line(writer, "503 5.5.1 AUTH not advertised by server").await?;
        return Ok(());
    };
    let mut parts = rest.splitn(2, ' ');
    let mech = parts.next().unwrap_or("").to_ascii_uppercase();
    let initial = parts.next().map(|s| s.trim().to_string());
    match mech.as_str() {
        "PLAIN" => {
            let payload = match initial {
                Some(b64) => b64,
                None => {
                    write_line(writer, "334 ").await?;
                    let line = match read_line(reader, 4096).await? {
                        Some(l) => l,
                        None => return Ok(()),
                    };
                    line.trim().to_string()
                }
            };
            let ok = decode_plain(&payload)
                .map(|(u, p)| u == user && p == pass)
                .unwrap_or(false);
            if ok {
                state.auth.authenticated = true;
                write_line(writer, "235 2.7.0 authentication successful").await?;
            } else {
                write_line(writer, "535 5.7.8 authentication failed").await?;
            }
        }
        "LOGIN" => {
            let supplied_user = match initial {
                Some(b64) => decode_b64_string(&b64),
                None => {
                    write_line(writer, "334 VXNlcm5hbWU6").await?; // "Username:"
                    let line = match read_line(reader, 4096).await? {
                        Some(l) => l,
                        None => return Ok(()),
                    };
                    decode_b64_string(line.trim())
                }
            };
            write_line(writer, "334 UGFzc3dvcmQ6").await?; // "Password:"
            let pass_line = match read_line(reader, 4096).await? {
                Some(l) => l,
                None => return Ok(()),
            };
            let supplied_pass = decode_b64_string(pass_line.trim());
            if supplied_user == user && supplied_pass == pass {
                state.auth.authenticated = true;
                write_line(writer, "235 2.7.0 authentication successful").await?;
            } else {
                write_line(writer, "535 5.7.8 authentication failed").await?;
            }
        }
        other => {
            write_line(
                writer,
                &format!("504 5.5.4 unrecognised AUTH mechanism: {other}"),
            )
            .await?;
        }
    }
    Ok(())
}

fn decode_plain(b64: &str) -> Option<(String, String)> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let mut parts = text.split('\0');
    let _authzid = parts.next()?;
    let user = parts.next()?.to_string();
    let pass = parts.next()?.to_string();
    Some((user, pass))
}

fn decode_b64_string(s: &str) -> String {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default()
}

async fn read_line<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    cap: usize,
) -> Result<Option<String>> {
    let mut buf = String::new();
    let mut total = 0usize;
    loop {
        let n = reader.read_line(&mut buf).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            return Ok(Some(buf));
        }
        total += n;
        if buf.ends_with('\n') {
            break;
        }
        if total > cap {
            anyhow::bail!("line exceeded {cap} bytes");
        }
    }
    Ok(Some(buf))
}

async fn write_line<W: AsyncWrite + Unpin>(writer: &mut W, line: &str) -> Result<()> {
    writer.write_all(line.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;
    writer.flush().await?;
    Ok(())
}

fn split_verb(line: &str) -> (&str, &str) {
    match line.split_once([' ', '\t']) {
        Some((v, rest)) => (v, rest.trim_start()),
        None => (line, ""),
    }
}

/// Pull the address out of `<addr> [SP <param>...]` or `addr [SP <param>...]`.
/// Returns `None` if the angle brackets are unbalanced or the input is empty.
/// Empty `<>` returns `Some("")` (the SMTP "null" sender).
pub fn extract_address(s: &str) -> Option<String> {
    let s = s.trim_start_matches([' ', ':']).trim();
    if let Some(stripped) = s.strip_prefix('<') {
        let end = stripped.find('>')?;
        return Some(stripped[..end].to_string());
    }
    let first = s.split_whitespace().next().unwrap_or("");
    if first.is_empty() {
        return None;
    }
    Some(first.to_string())
}

/// Read DATA payload until "\r\n.\r\n" terminator. Performs dot-unstuffing per
/// RFC 5321 §4.5.2 (a leading "." on a line is dropped).
pub async fn read_data_body<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    max: usize,
) -> Result<DataOutcome> {
    let mut buf = BytesMut::new();
    let mut size_exceeded = false;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(DataOutcome::Closed);
        }
        if line == ".\r\n" || line == ".\n" {
            return Ok(if size_exceeded {
                DataOutcome::TooLarge
            } else {
                DataOutcome::Done(buf.freeze())
            });
        }
        let unstuffed = if let Some(rest) = line.strip_prefix('.') {
            rest
        } else {
            line.as_str()
        };
        if buf.len() + unstuffed.len() > max {
            size_exceeded = true;
            continue;
        }
        buf.extend_from_slice(unstuffed.as_bytes());
    }
}

#[derive(Debug)]
pub enum DataOutcome {
    Done(Bytes),
    TooLarge,
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncReadExt};

    fn cfg() -> SmtpConfig {
        SmtpConfig::default()
    }

    fn cfg_with_auth() -> SmtpConfig {
        SmtpConfig {
            auth: Some(("alice".into(), "s3cret".into())),
            ..SmtpConfig::default()
        }
    }

    fn peer() -> SocketAddr {
        "127.0.0.1:1".parse().unwrap()
    }

    #[test]
    fn extract_address_handles_angle_and_bare_forms() {
        assert_eq!(extract_address("<a@b>").as_deref(), Some("a@b"));
        assert_eq!(extract_address(": <a@b>").as_deref(), Some("a@b"));
        assert_eq!(extract_address("<>").as_deref(), Some(""));
        assert_eq!(extract_address("a@b").as_deref(), Some("a@b"));
        assert_eq!(extract_address("a@b SIZE=1234").as_deref(), Some("a@b"));
        assert!(extract_address("<unbalanced").is_none());
        assert!(extract_address("").is_none());
    }

    #[test]
    fn split_verb_splits_on_first_whitespace() {
        assert_eq!(split_verb("HELO localhost"), ("HELO", "localhost"));
        assert_eq!(split_verb("NOOP"), ("NOOP", ""));
        assert_eq!(split_verb("MAIL FROM:<a@b>"), ("MAIL", "FROM:<a@b>"));
    }

    #[test]
    fn smtp_config_default_sane() {
        let c = cfg();
        assert_eq!(c.hostname, "MailBoxUltra");
        assert_eq!(c.max_message_size, 25 * 1024 * 1024);
        assert!(c.auth.is_none());
    }

    #[tokio::test]
    async fn read_data_body_dot_unstuffs_and_terminates() {
        let raw = "hello\r\n..world\r\n.\r\n";
        let mut r = BufReader::new(raw.as_bytes());
        match read_data_body(&mut r, 1024).await.unwrap() {
            DataOutcome::Done(b) => {
                assert_eq!(&b[..], b"hello\r\n.world\r\n");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_data_body_size_exceeded_keeps_consuming() {
        let raw = "aaaa\r\nbbbb\r\n.\r\n";
        let mut r = BufReader::new(raw.as_bytes());
        match read_data_body(&mut r, 4).await.unwrap() {
            DataOutcome::TooLarge => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_data_body_closed_when_no_terminator() {
        let raw = "incomplete";
        let mut r = BufReader::new(raw.as_bytes());
        match read_data_body(&mut r, 1024).await.unwrap() {
            DataOutcome::Closed => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn decode_plain_extracts_user_and_pass() {
        // "\0alice\0s3cret"
        let payload = base64::engine::general_purpose::STANDARD.encode(b"\0alice\0s3cret");
        let (u, p) = decode_plain(&payload).unwrap();
        assert_eq!(u, "alice");
        assert_eq!(p, "s3cret");
    }

    #[test]
    fn decode_plain_rejects_garbage() {
        assert!(decode_plain("not-base64!!").is_none());
    }

    #[test]
    fn decode_b64_string_is_lossy_safe() {
        assert_eq!(decode_b64_string("YWxpY2U="), "alice");
        assert_eq!(decode_b64_string("?invalid?"), "");
    }

    /// Drive a complete session through an in-memory duplex pipe and return
    /// everything the server wrote.
    async fn drive(commands: &str, cfg: SmtpConfig) -> (String, Arc<MessageStore>) {
        let (server_io, client_io) = duplex(64 * 1024);
        let (cr, mut cw) = tokio::io::split(client_io);
        let (sr, sw) = tokio::io::split(server_io);
        let store = MessageStore::new(16);

        let store_clone = store.clone();
        let cfg_clone = cfg.clone();
        let server_task = tokio::spawn(async move {
            let _ = run_session(BufReader::new(sr), sw, peer(), store_clone, cfg_clone).await;
        });

        // Feed commands then close the write half.
        let cmds = commands.replace('\n', "\r\n");
        cw.write_all(cmds.as_bytes()).await.unwrap();
        cw.shutdown().await.unwrap();

        let mut output = String::new();
        let mut cr = cr;
        cr.read_to_string(&mut output).await.unwrap();
        let _ = server_task.await;
        (output, store)
    }

    #[tokio::test]
    async fn full_session_captures_a_message() {
        let cmds = "EHLO test\n\
             MAIL FROM:<a@x>\n\
             RCPT TO:<b@x>\n\
             DATA\n\
             Subject: Hi\n\
             From: a@x\n\
             To: b@x\n\
             \n\
             body line\n\
             ..dot stuffed\n\
             .\n\
             QUIT\n";
        let (out, store) = drive(cmds, cfg()).await;
        assert!(out.contains("220"), "missing greeting: {out}");
        assert!(
            out.contains("250 2.0.0 message accepted"),
            "no 250 ok: {out}"
        );
        assert!(out.contains("221 2.0.0"));
        let msgs = store.list(10);
        assert_eq!(msgs.len(), 1);
        let m = &msgs[0];
        assert_eq!(m.envelope_from, "a@x");
        assert_eq!(m.envelope_to, vec!["b@x".to_string()]);
        assert_eq!(m.subject.as_deref(), Some("Hi"));
        let txt = m.text.as_deref().unwrap_or("");
        assert!(txt.contains("body line"), "body lost: {txt}");
        assert!(txt.contains(".dot stuffed"), "dot-unstuffing broken: {txt}");
    }

    #[tokio::test]
    async fn helo_then_session_works() {
        let cmds = "HELO test\nMAIL FROM:<a@x>\nRCPT TO:<b@x>\nDATA\nSubject: H\n\nb\n.\nQUIT\n";
        let (out, store) = drive(cmds, cfg()).await;
        assert!(out.contains("250 MailBoxUltra hello"));
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn ehlo_advertises_size_and_pipelining() {
        let (out, _) = drive("EHLO me\nQUIT\n", cfg()).await;
        assert!(out.contains("250-PIPELINING"));
        assert!(out.contains("250-8BITMIME"));
        assert!(out.contains("250-SMTPUTF8"));
        assert!(out.contains("250-SIZE 26214400"));
        assert!(!out.contains("AUTH"));
    }

    #[tokio::test]
    async fn ehlo_advertises_auth_when_configured() {
        let (out, _) = drive("EHLO me\nQUIT\n", cfg_with_auth()).await;
        assert!(out.contains("250-AUTH PLAIN LOGIN"));
    }

    #[tokio::test]
    async fn rejects_mail_before_helo() {
        let (out, _) = drive("MAIL FROM:<a@x>\nQUIT\n", cfg()).await;
        assert!(out.contains("503 5.5.1 send HELO/EHLO first"));
    }

    #[tokio::test]
    async fn rejects_rcpt_before_mail() {
        let (out, _) = drive("EHLO me\nRCPT TO:<b@x>\nQUIT\n", cfg()).await;
        assert!(out.contains("503 5.5.1 need MAIL FROM first"));
    }

    #[tokio::test]
    async fn rejects_data_with_no_envelope() {
        let (out, _) = drive("EHLO me\nDATA\nQUIT\n", cfg()).await;
        assert!(out.contains("503 5.5.1 need MAIL FROM and RCPT TO first"));
    }

    #[tokio::test]
    async fn auth_required_blocks_mail() {
        let (out, store) = drive("EHLO me\nMAIL FROM:<a@x>\nQUIT\n", cfg_with_auth()).await;
        assert!(out.contains("530 5.7.0 authentication required"));
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn auth_plain_accepts_correct_credentials() {
        let creds = base64::engine::general_purpose::STANDARD.encode(b"\0alice\0s3cret");
        let cmds = format!(
            "EHLO me\nAUTH PLAIN {creds}\nMAIL FROM:<a@x>\nRCPT TO:<b@x>\nDATA\nSubject: H\n\nb\n.\nQUIT\n"
        );
        let (out, store) = drive(&cmds, cfg_with_auth()).await;
        assert!(out.contains("235 2.7.0 authentication successful"), "{out}");
        assert!(out.contains("250 2.0.0 message accepted"));
        assert_eq!(store.len(), 1);
        assert!(store.list(10)[0].authenticated);
    }

    #[tokio::test]
    async fn auth_plain_rejects_wrong_password() {
        let creds = base64::engine::general_purpose::STANDARD.encode(b"\0alice\0nope");
        let cmds = format!("EHLO me\nAUTH PLAIN {creds}\nQUIT\n");
        let (out, _) = drive(&cmds, cfg_with_auth()).await;
        assert!(out.contains("535 5.7.8 authentication failed"));
    }

    #[tokio::test]
    async fn auth_plain_two_step_prompt() {
        // Client sends `AUTH PLAIN`, server replies `334 `, client sends b64 creds.
        let creds = base64::engine::general_purpose::STANDARD.encode(b"\0alice\0s3cret");
        let cmds = format!("EHLO me\nAUTH PLAIN\n{creds}\nQUIT\n");
        let (out, _) = drive(&cmds, cfg_with_auth()).await;
        assert!(out.contains("334 "));
        assert!(out.contains("235 2.7.0 authentication successful"));
    }

    #[tokio::test]
    async fn auth_login_two_step_prompt() {
        let user = base64::engine::general_purpose::STANDARD.encode(b"alice");
        let pass = base64::engine::general_purpose::STANDARD.encode(b"s3cret");
        let cmds = format!("EHLO me\nAUTH LOGIN\n{user}\n{pass}\nQUIT\n");
        let (out, _) = drive(&cmds, cfg_with_auth()).await;
        assert!(out.contains("334 VXNlcm5hbWU6"));
        assert!(out.contains("334 UGFzc3dvcmQ6"));
        assert!(out.contains("235 2.7.0 authentication successful"));
    }

    #[tokio::test]
    async fn auth_login_inline_user() {
        let user = base64::engine::general_purpose::STANDARD.encode(b"alice");
        let pass = base64::engine::general_purpose::STANDARD.encode(b"s3cret");
        let cmds = format!("EHLO me\nAUTH LOGIN {user}\n{pass}\nQUIT\n");
        let (out, _) = drive(&cmds, cfg_with_auth()).await;
        assert!(out.contains("334 UGFzc3dvcmQ6"));
        assert!(out.contains("235"));
    }

    #[tokio::test]
    async fn auth_unknown_mechanism_rejected() {
        let (out, _) = drive("EHLO me\nAUTH GSSAPI\nQUIT\n", cfg_with_auth()).await;
        assert!(out.contains("504 5.5.4 unrecognised AUTH mechanism: GSSAPI"));
    }

    #[tokio::test]
    async fn auth_when_not_advertised_returns_503() {
        let (out, _) = drive("EHLO me\nAUTH PLAIN AA==\nQUIT\n", cfg()).await;
        assert!(out.contains("503 5.5.1 AUTH not advertised by server"));
    }

    #[tokio::test]
    async fn rset_clears_envelope() {
        let cmds = "EHLO me\nMAIL FROM:<a@x>\nRSET\nMAIL FROM:<c@x>\nRCPT TO:<d@x>\nDATA\nS: 1\n\nb\n.\nQUIT\n";
        let (out, store) = drive(cmds, cfg()).await;
        assert!(out.contains("250 2.0.0 OK"));
        assert_eq!(store.len(), 1);
        assert_eq!(store.list(10)[0].envelope_from, "c@x");
    }

    #[tokio::test]
    async fn noop_help_vrfy() {
        let (out, _) = drive("EHLO me\nNOOP\nHELP\nVRFY user@host\nQUIT\n", cfg()).await;
        assert!(out.contains("250 2.0.0 OK"));
        assert!(out.contains("214 2.0.0 Commands:"));
        assert!(out.contains("252 2.5.2"));
    }

    #[tokio::test]
    async fn unknown_verb_returns_500() {
        let (out, _) = drive("EHLO me\nFOOBAR\nQUIT\n", cfg()).await;
        assert!(out.contains("500 5.5.2 unrecognised command: FOOBAR"));
    }

    #[tokio::test]
    async fn empty_command_returns_500() {
        let (out, _) = drive("EHLO me\n\nQUIT\n", cfg()).await;
        assert!(out.contains("500 5.5.2 syntax error: empty command"));
    }

    #[tokio::test]
    async fn invalid_mail_syntax_returns_501() {
        let (out, _) = drive("EHLO me\nMAIL whoops\nQUIT\n", cfg()).await;
        assert!(out.contains("501 5.5.4 syntax: MAIL FROM:<address>"));
    }

    #[tokio::test]
    async fn invalid_rcpt_syntax_returns_501() {
        let (out, _) = drive("EHLO me\nMAIL FROM:<a@x>\nRCPT whoops\nQUIT\n", cfg()).await;
        assert!(out.contains("501 5.5.4 syntax: RCPT TO:<address>"));
    }

    #[tokio::test]
    async fn empty_rcpt_address_rejected() {
        let (out, _) = drive("EHLO me\nMAIL FROM:<a@x>\nRCPT TO:<>\nQUIT\n", cfg()).await;
        assert!(out.contains("501 5.5.4 invalid recipient address"));
    }

    #[tokio::test]
    async fn data_too_large_returns_552() {
        let mut cfg = cfg();
        cfg.max_message_size = 8;
        let cmds = "EHLO me\nMAIL FROM:<a@x>\nRCPT TO:<b@x>\nDATA\nthis is more than eight bytes\n.\nQUIT\n";
        let (out, store) = drive(cmds, cfg).await;
        assert!(out.contains("552 5.3.4 message exceeds size limit"));
        assert_eq!(store.len(), 0);
    }
}
