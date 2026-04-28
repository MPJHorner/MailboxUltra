//! Optional relay: after a captured message lands in the store, hand it off
//! to a real upstream MTA. Configured via `--relay smtp://host:port`.
//!
//! The relay runs as a separate task that consumes the broadcast channel; a
//! relay failure never blocks capture or causes the original sender to see an
//! error -- it is logged and the message stays in the store.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::message::Message;
use crate::store::{MessageStore, StoreEvent};

/// Parsed-and-validated relay configuration.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub url: url::Url,
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub insecure: bool,
    pub timeout: Duration,
    pub auth: Option<(String, String)>,
}

impl RelayConfig {
    pub fn from_url(url: url::Url, insecure: bool) -> Result<Self> {
        let scheme = url.scheme().to_ascii_lowercase();
        let use_tls = match scheme.as_str() {
            "smtp" => false,
            "smtps" => true,
            other => bail!("relay URL must use smtp or smtps, got '{other}'"),
        };
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("relay URL missing host"))?
            .to_string();
        let port = url.port().unwrap_or(if use_tls { 465 } else { 25 });
        let auth = if !url.username().is_empty() {
            Some((
                url.username().to_string(),
                url.password().unwrap_or("").to_string(),
            ))
        } else {
            None
        };
        Ok(Self {
            url,
            host,
            port,
            use_tls,
            insecure,
            timeout: Duration::from_secs(30),
            auth,
        })
    }
}

pub type RelaySwitch = Arc<RwLock<Option<RelayConfig>>>;

pub fn new_switch(initial: Option<RelayConfig>) -> RelaySwitch {
    Arc::new(RwLock::new(initial))
}

/// Spawn the relay task. It exits when the broadcast channel closes.
pub fn spawn_relay(store: Arc<MessageStore>, switch: RelaySwitch) -> JoinHandle<()> {
    let mut rx = store.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(StoreEvent::Message(msg)) => {
                    let snap = switch.read().await.clone();
                    if let Some(cfg) = snap {
                        if let Err(e) = relay_message(&cfg, &msg).await {
                            tracing::warn!(error = %e, "relay failed");
                        }
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

/// Relay a single message via plain SMTP. The caller already validated the
/// URL; here we open a TCP connection, walk the SMTP exchange, and ship the
/// raw RFC 822 body. TLS is reserved for a future release; passing an
/// `smtps://` URL currently returns an error so the user knows it isn't
/// silently downgraded.
pub async fn relay_message(cfg: &RelayConfig, msg: &Message) -> Result<()> {
    if cfg.use_tls {
        bail!("smtps:// relay is not yet implemented; use smtp:// or open a tracking issue");
    }
    let _ = cfg.insecure; // reserved for the smtps path
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let stream = tokio::time::timeout(cfg.timeout, TcpStream::connect(&addr))
        .await
        .with_context(|| format!("connecting to relay {addr}: timeout"))?
        .with_context(|| format!("connecting to relay {addr}"))?;
    stream.set_nodelay(true).ok();
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);

    expect_code(&mut reader, 220).await?;

    write_line(&mut write, "EHLO mailbox-ultra").await?;
    drain_multiline(&mut reader, 250).await?;

    if let Some((user, pass)) = &cfg.auth {
        let payload = format!("\0{user}\0{pass}");
        let b64 = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
        write_line(&mut write, &format!("AUTH PLAIN {b64}")).await?;
        expect_code(&mut reader, 235).await?;
    }

    write_line(&mut write, &format!("MAIL FROM:<{}>", msg.envelope_from)).await?;
    expect_code(&mut reader, 250).await?;

    for rcpt in &msg.envelope_to {
        write_line(&mut write, &format!("RCPT TO:<{rcpt}>")).await?;
        expect_code(&mut reader, 250).await?;
    }

    write_line(&mut write, "DATA").await?;
    expect_code(&mut reader, 354).await?;

    // Dot-stuff and write raw bytes.
    let stuffed = dot_stuff(&msg.raw);
    write.write_all(&stuffed).await?;
    write.write_all(b"\r\n.\r\n").await?;
    write.flush().await?;
    expect_code(&mut reader, 250).await?;

    write_line(&mut write, "QUIT").await?;
    drain_multiline(&mut reader, 221).await.ok();
    Ok(())
}

use base64::Engine;

async fn write_line<W: AsyncWriteExt + Unpin>(w: &mut W, s: &str) -> Result<()> {
    w.write_all(s.as_bytes()).await?;
    w.write_all(b"\r\n").await?;
    w.flush().await?;
    Ok(())
}

/// Read a multi-line reply and assert the final code matches `expected`.
async fn drain_multiline<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
    expected: u16,
) -> Result<()> {
    loop {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf).await?;
        if n == 0 {
            bail!("connection closed waiting for {expected}");
        }
        if buf.len() < 4 {
            bail!("malformed reply: {buf:?}");
        }
        let code: u16 = buf[..3]
            .parse()
            .with_context(|| format!("bad SMTP code: {buf:?}"))?;
        let last = buf.as_bytes().get(3).copied() == Some(b' ');
        if last {
            if code != expected {
                bail!("expected {expected}, got {code}: {}", buf.trim_end());
            }
            return Ok(());
        }
    }
}

async fn expect_code<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
    expected: u16,
) -> Result<()> {
    drain_multiline(reader, expected).await
}

/// Dot-stuff a body: any line beginning with "." gets a "." prepended (the
/// inverse of the unstuffing the receiver does). Lines are split on \r\n; if
/// the input uses bare \n we normalise to \r\n on the wire.
pub fn dot_stuff(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut at_line_start = true;
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        if at_line_start && b == b'.' {
            out.push(b'.');
        }
        if b == b'\n' && (i == 0 || body[i - 1] != b'\r') {
            out.push(b'\r');
        }
        out.push(b);
        at_line_start = b == b'\n';
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_url_extracts_host_port_and_creds() {
        let u = url::Url::parse("smtp://alice:secret@mail.example.com:2525").unwrap();
        let cfg = RelayConfig::from_url(u, false).unwrap();
        assert_eq!(cfg.host, "mail.example.com");
        assert_eq!(cfg.port, 2525);
        assert!(!cfg.use_tls);
        assert_eq!(cfg.auth, Some(("alice".into(), "secret".into())));
    }

    #[test]
    fn from_url_smtps_default_port_465() {
        let u = url::Url::parse("smtps://relay.example.com").unwrap();
        let cfg = RelayConfig::from_url(u, false).unwrap();
        assert_eq!(cfg.port, 465);
        assert!(cfg.use_tls);
    }

    #[test]
    fn from_url_smtp_default_port_25() {
        let u = url::Url::parse("smtp://relay.example.com").unwrap();
        let cfg = RelayConfig::from_url(u, false).unwrap();
        assert_eq!(cfg.port, 25);
        assert!(!cfg.use_tls);
        assert!(cfg.auth.is_none());
    }

    #[test]
    fn from_url_rejects_unknown_scheme() {
        let u = url::Url::parse("http://relay.example.com").unwrap();
        let err = RelayConfig::from_url(u, false).unwrap_err();
        assert!(err.to_string().contains("smtp or smtps"));
    }

    #[test]
    fn from_url_rejects_missing_host() {
        // relative URL constructions like `smtp:///` parse to host=""
        let u = url::Url::parse("smtp:///path").unwrap();
        let err = RelayConfig::from_url(u, false).unwrap_err();
        assert!(err.to_string().contains("missing host"));
    }

    #[test]
    fn dot_stuff_prefixes_lines_starting_with_dot() {
        assert_eq!(dot_stuff(b"hi\r\n.test\r\n"), b"hi\r\n..test\r\n");
        assert_eq!(dot_stuff(b".only\r\n"), b"..only\r\n");
        assert_eq!(dot_stuff(b"none\r\n"), b"none\r\n");
    }

    #[test]
    fn dot_stuff_normalises_bare_lf() {
        assert_eq!(dot_stuff(b"a\nb\n"), b"a\r\nb\r\n");
    }

    #[tokio::test]
    async fn smtps_relay_returns_clear_error() {
        let cfg = RelayConfig::from_url(
            url::Url::parse("smtps://example.invalid:465").unwrap(),
            false,
        )
        .unwrap();
        let msg = mk_msg();
        let err = relay_message(&cfg, &msg).await.unwrap_err();
        assert!(err.to_string().contains("smtps:// relay"));
    }

    #[tokio::test]
    async fn relay_against_local_fake_succeeds() {
        // Spin up a tiny stub that walks the standard SMTP exchange and accepts
        // whatever we send.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            let (r, mut w) = sock.into_split();
            let mut r = BufReader::new(r);
            w.write_all(b"220 stub ready\r\n").await.unwrap();
            for reply in [
                "250-stub\r\n250 OK\r\n",
                "250 ok sender\r\n",
                "250 ok rcpt\r\n",
                "354 go ahead\r\n",
            ] {
                let mut line = String::new();
                r.read_line(&mut line).await.unwrap();
                w.write_all(reply.as_bytes()).await.unwrap();
            }
            // Read body until "\r\n.\r\n"
            let mut buf = vec![0u8; 1];
            let mut tail = Vec::new();
            loop {
                if r.read_exact(&mut buf).await.is_err() {
                    break;
                }
                tail.push(buf[0]);
                if tail.ends_with(b"\r\n.\r\n") {
                    break;
                }
            }
            w.write_all(b"250 ok body\r\n").await.unwrap();
            // QUIT
            let mut line = String::new();
            let _ = r.read_line(&mut line).await;
            w.write_all(b"221 bye\r\n").await.unwrap();
        });
        let cfg = RelayConfig::from_url(
            url::Url::parse(&format!("smtp://127.0.0.1:{}", addr.port())).unwrap(),
            false,
        )
        .unwrap();
        relay_message(&cfg, &mk_msg()).await.unwrap();
    }

    #[tokio::test]
    async fn relay_against_local_fake_with_auth() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            let (r, mut w) = sock.into_split();
            let mut r = BufReader::new(r);
            w.write_all(b"220 stub ready\r\n").await.unwrap();
            for reply in [
                "250-stub\r\n250 AUTH PLAIN\r\n",
                "235 ok auth\r\n",
                "250 ok sender\r\n",
                "250 ok rcpt\r\n",
                "354 go ahead\r\n",
            ] {
                let mut line = String::new();
                r.read_line(&mut line).await.unwrap();
                w.write_all(reply.as_bytes()).await.unwrap();
            }
            let mut buf = vec![0u8; 1];
            let mut tail = Vec::new();
            loop {
                if r.read_exact(&mut buf).await.is_err() {
                    break;
                }
                tail.push(buf[0]);
                if tail.ends_with(b"\r\n.\r\n") {
                    break;
                }
            }
            w.write_all(b"250 ok body\r\n").await.unwrap();
            let mut line = String::new();
            let _ = r.read_line(&mut line).await;
            w.write_all(b"221 bye\r\n").await.unwrap();
        });
        let cfg = RelayConfig::from_url(
            url::Url::parse(&format!("smtp://alice:secret@127.0.0.1:{}", addr.port())).unwrap(),
            false,
        )
        .unwrap();
        relay_message(&cfg, &mk_msg()).await.unwrap();
    }

    #[tokio::test]
    async fn relay_handles_unexpected_code() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            let (_, mut w) = sock.into_split();
            // Send a 421 instead of 220.
            w.write_all(b"421 too busy\r\n").await.unwrap();
        });
        let cfg = RelayConfig::from_url(
            url::Url::parse(&format!("smtp://127.0.0.1:{}", addr.port())).unwrap(),
            false,
        )
        .unwrap();
        let err = relay_message(&cfg, &mk_msg()).await.unwrap_err();
        assert!(err.to_string().contains("expected 220"));
    }

    fn mk_msg() -> Message {
        crate::message::parse_message(
            bytes::Bytes::from_static(b"From: a@x\r\nTo: b@x\r\nSubject: Hi\r\n\r\nbody\r\n"),
            "a@x".into(),
            vec!["b@x".into()],
            "1.1.1.1:1".into(),
            false,
        )
    }
}
