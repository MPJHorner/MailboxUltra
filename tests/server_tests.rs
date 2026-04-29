//! End-to-end server tests. Bring up `ServerHandle` against a tokio runtime,
//! drive real SMTP traffic through it with `lettre`, and assert that the
//! MessageStore captured what we sent. These tests cover what the old
//! `tests/smtp_tests.rs` and `tests/e2e_tests.rs` covered, but without the
//! deleted CLI / web layers.

use std::time::Duration;

use lettre::{
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::{authentication::Credentials, client::Tls},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use mailbox_ultra::{
    server::ServerHandle,
    settings::{Auth, PersistentSettings, RelaySettings},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn ephemeral() -> PersistentSettings {
    PersistentSettings {
        smtp_port: 0,
        buffer_size: 1000,
        ..PersistentSettings::default()
    }
}

fn transport(port: u16) -> AsyncSmtpTransport<Tokio1Executor> {
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
        .port(port)
        .tls(Tls::None)
        .timeout(Some(Duration::from_secs(5)))
        .build()
}

async fn wait_until<F>(mut pred: F)
where
    F: FnMut() -> bool,
{
    for _ in 0..200 {
        if pred() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for condition");
}

#[test]
fn lettre_can_send_a_simple_message() {
    let rt = rt();
    let handle = ServerHandle::start(rt.handle().clone(), ephemeral()).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let mailer = transport(port);
        let email = Message::builder()
            .from("Alice <alice@example.com>".parse().unwrap())
            .to("bob@example.com".parse().unwrap())
            .subject("hi from lettre")
            .header(ContentType::TEXT_PLAIN)
            .body(String::from("hello world"))
            .unwrap();
        mailer.send(email).await.unwrap();

        let store = handle.store();
        wait_until(|| store.len() == 1).await;
    });

    let msgs = handle.store().list(10);
    assert_eq!(msgs.len(), 1);
    let m = &msgs[0];
    assert_eq!(m.subject.as_deref(), Some("hi from lettre"));
    assert_eq!(m.envelope_from, "alice@example.com");
    assert_eq!(m.envelope_to, vec!["bob@example.com".to_string()]);
    assert!(m.text.as_deref().unwrap().contains("hello world"));
    handle.shutdown();
}

#[test]
fn html_and_text_alternative_captured() {
    let rt = rt();
    let handle = ServerHandle::start(rt.handle().clone(), ephemeral()).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let mailer = transport(port);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("alt")
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(String::from("plain body")),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(String::from("<p>html body</p>")),
                    ),
            )
            .unwrap();
        mailer.send(email).await.unwrap();

        let store = handle.store();
        wait_until(|| store.len() == 1).await;
    });

    let m = handle.store().list(10).remove(0);
    assert!(m.text.as_deref().unwrap().contains("plain body"));
    assert!(m.html.as_deref().unwrap().contains("html body"));
    handle.shutdown();
}

#[test]
fn attachment_preserved() {
    let rt = rt();
    let handle = ServerHandle::start(rt.handle().clone(), ephemeral()).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let mailer = transport(port);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("with attachment")
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(String::from("see attached")))
                    .singlepart(
                        Attachment::new(String::from("file.txt"))
                            .body(String::from("hello attachment"), ContentType::TEXT_PLAIN),
                    ),
            )
            .unwrap();
        mailer.send(email).await.unwrap();

        let store = handle.store();
        wait_until(|| store.len() == 1).await;
    });

    let m = handle.store().list(10).remove(0);
    assert_eq!(m.attachments.len(), 1);
    assert_eq!(m.attachments[0].filename.as_deref(), Some("file.txt"));
    handle.shutdown();
}

#[test]
fn auth_required_rejects_unauthenticated_and_accepts_correct_credentials() {
    let rt = rt();
    let mut s = ephemeral();
    s.auth = Some(Auth {
        user: "alice".into(),
        pass: "s3cret".into(),
    });
    let handle = ServerHandle::start(rt.handle().clone(), s).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let bad = transport(port);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("nope")
            .body(String::from("body"))
            .unwrap();
        let res = bad.send(email).await;
        assert!(res.is_err(), "expected auth error");

        let good = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .tls(Tls::None)
            .credentials(Credentials::new("alice".into(), "s3cret".into()))
            .timeout(Some(Duration::from_secs(5)))
            .build();
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("authed")
            .body(String::from("body"))
            .unwrap();
        good.send(email).await.unwrap();

        let store = handle.store();
        wait_until(|| store.len() == 1).await;
    });

    let m = handle.store().list(10).remove(0);
    assert!(m.authenticated);
    assert_eq!(m.subject.as_deref(), Some("authed"));
    handle.shutdown();
}

#[test]
fn message_too_large_rejected() {
    let rt = rt();
    let mut s = ephemeral();
    s.max_message_size = 256;
    let handle = ServerHandle::start(rt.handle().clone(), s).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let mailer = transport(port);
        let body = "X".repeat(2000);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("big")
            .body(body)
            .unwrap();
        let res = mailer.send(email).await;
        assert!(res.is_err(), "expected oversize rejection");
    });

    assert_eq!(handle.store().len(), 0);
    handle.shutdown();
}

#[test]
fn log_file_appends_ndjson_per_message() {
    let rt = rt();
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("mbu.log");

    let mut s = ephemeral();
    s.log_file = Some(path.clone());
    let handle = ServerHandle::start(rt.handle().clone(), s).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        for subject in ["logged-1", "logged-2"] {
            let mailer = transport(port);
            let email = Message::builder()
                .from("a@x".parse().unwrap())
                .to("b@x".parse().unwrap())
                .subject(subject)
                .body(format!("body for {subject}"))
                .unwrap();
            mailer.send(email).await.unwrap();
        }

        for _ in 0..200 {
            if let Ok(s) = tokio::fs::read_to_string(&path).await {
                if s.lines().count() >= 2 {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("log file did not receive 2 lines in time");
    });

    let body = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    let v1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let v2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let subjects = [
        v1["subject"].as_str().unwrap_or(""),
        v2["subject"].as_str().unwrap_or(""),
    ];
    assert!(subjects.contains(&"logged-1"));
    assert!(subjects.contains(&"logged-2"));
    handle.shutdown();
}

#[test]
fn relay_task_forwards_to_stub_upstream() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let rt = rt();

    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let upstream_addr = rt.block_on(async move {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = upstream.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((sock, _)) = upstream.accept().await {
                let received = received_clone.clone();
                tokio::spawn(async move {
                    let (r, mut w) = sock.into_split();
                    let mut r = BufReader::new(r);
                    let _ = w.write_all(b"220 stub ready\r\n").await;
                    let mut line = String::new();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"250-stub\r\n250 OK\r\n").await;
                    line.clear();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"250 ok sender\r\n").await;
                    line.clear();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"250 ok rcpt\r\n").await;
                    line.clear();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"354 go ahead\r\n").await;
                    let mut body = Vec::new();
                    let mut buf = [0u8; 1];
                    loop {
                        if r.read_exact(&mut buf).await.is_err() {
                            break;
                        }
                        body.push(buf[0]);
                        if body.ends_with(b"\r\n.\r\n") {
                            break;
                        }
                    }
                    received
                        .lock()
                        .await
                        .push(String::from_utf8_lossy(&body).into_owned());
                    let _ = w.write_all(b"250 ok body\r\n").await;
                    line.clear();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"221 bye\r\n").await;
                });
            }
        });
        addr
    });

    let mut s = ephemeral();
    s.relay = Some(RelaySettings {
        url: format!("smtp://{}", upstream_addr),
        insecure: false,
    });
    let handle = ServerHandle::start(rt.handle().clone(), s).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        let mailer = transport(port);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("relayed")
            .body(String::from("body"))
            .unwrap();
        mailer.send(email).await.unwrap();

        for _ in 0..200 {
            if !received.lock().await.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("upstream did not receive the relayed body");
    });

    let got = rt.block_on(async { received.lock().await.clone() });
    assert!(!got.is_empty());
    assert!(got[0].contains("Subject: relayed"), "{got:?}");
    handle.shutdown();
}

#[test]
fn buffer_size_evicts_oldest_messages() {
    let rt = rt();
    let mut s = ephemeral();
    s.buffer_size = 3;
    let handle = ServerHandle::start(rt.handle().clone(), s).unwrap();
    let port = handle.smtp_addr().port();

    rt.block_on(async {
        for subject in ["a", "b", "c", "d", "e"] {
            let mailer = transport(port);
            let email = Message::builder()
                .from("a@x".parse().unwrap())
                .to("b@x".parse().unwrap())
                .subject(subject)
                .body(format!("body for {subject}"))
                .unwrap();
            mailer.send(email).await.unwrap();
        }

        let store = handle.store();
        wait_until(|| store.len() == 3).await;
    });

    let subjects: Vec<String> = handle
        .store()
        .list(10)
        .into_iter()
        .map(|m| m.subject.unwrap_or_default())
        .collect();
    assert_eq!(
        subjects,
        vec!["e".to_string(), "d".to_string(), "c".to_string()]
    );
    handle.shutdown();
}

#[test]
fn restart_relay_via_handle_starts_forwarding_without_smtp_restart() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let rt = rt();
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let upstream_addr = rt.block_on(async move {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = upstream.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((sock, _)) = upstream.accept().await {
                let received = received_clone.clone();
                tokio::spawn(async move {
                    let (r, mut w) = sock.into_split();
                    let mut r = BufReader::new(r);
                    let _ = w.write_all(b"220 stub ready\r\n").await;
                    for reply in [
                        "250-stub\r\n250 OK\r\n",
                        "250 ok sender\r\n",
                        "250 ok rcpt\r\n",
                        "354 go ahead\r\n",
                    ] {
                        let mut line = String::new();
                        let _ = r.read_line(&mut line).await;
                        let _ = w.write_all(reply.as_bytes()).await;
                    }
                    let mut body = Vec::new();
                    let mut buf = [0u8; 1];
                    loop {
                        if r.read_exact(&mut buf).await.is_err() {
                            break;
                        }
                        body.push(buf[0]);
                        if body.ends_with(b"\r\n.\r\n") {
                            break;
                        }
                    }
                    received
                        .lock()
                        .await
                        .push(String::from_utf8_lossy(&body).into_owned());
                    let _ = w.write_all(b"250 ok body\r\n").await;
                    let mut line = String::new();
                    let _ = r.read_line(&mut line).await;
                    let _ = w.write_all(b"221 bye\r\n").await;
                });
            }
        });
        addr
    });

    let handle = ServerHandle::start(rt.handle().clone(), ephemeral()).unwrap();
    let port_before = handle.smtp_addr().port();

    let mut new_settings = handle.settings();
    new_settings.relay = Some(RelaySettings {
        url: format!("smtp://{}", upstream_addr),
        insecure: false,
    });
    let report = handle.restart(new_settings).unwrap();
    assert!(report.relay_changed);
    assert!(report.smtp_restarted.is_none());
    assert_eq!(handle.smtp_addr().port(), port_before);

    rt.block_on(async {
        let mailer = transport(port_before);
        let email = Message::builder()
            .from("a@x".parse().unwrap())
            .to("b@x".parse().unwrap())
            .subject("after-relay-toggle")
            .body(String::from("body"))
            .unwrap();
        mailer.send(email).await.unwrap();

        for _ in 0..200 {
            if !received.lock().await.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("upstream did not receive the relayed body after hot-update");
    });

    handle.shutdown();
}
