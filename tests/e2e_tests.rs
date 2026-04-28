//! Cross-cutting end-to-end checks: NDJSON log file, in-process relay against
//! a stub upstream, and the printer/banner pipeline.

use std::time::Duration;

use clap::Parser;
use lettre::{
    message::header::ContentType, transport::smtp::client::Tls, AsyncSmtpTransport, AsyncTransport,
    Message, Tokio1Executor,
};
use mailbox_ultra::{
    app,
    cli::Cli,
    output::{Printer, PrinterOptions},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

fn quiet_printer() -> Printer {
    Printer::new(PrinterOptions {
        use_color: false,
        json_mode: false,
        verbose: false,
        quiet: true,
    })
}

fn cli(args: &[&str]) -> Cli {
    let mut v = vec!["mailbox-ultra"];
    v.extend_from_slice(args);
    Cli::parse_from(v)
}

async fn send_one(port: u16, subject: &str) {
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
        .port(port)
        .tls(Tls::None)
        .timeout(Some(Duration::from_secs(5)))
        .build();
    let email = Message::builder()
        .from("a@x".parse().unwrap())
        .to("b@x".parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(format!("body for {subject}"))
        .unwrap();
    mailer.send(email).await.unwrap();
}

#[tokio::test]
async fn log_file_appends_ndjson_per_message() {
    let path = std::env::temp_dir().join(format!("mbu-e2e-{}.log", uuid::Uuid::new_v4()));
    let path_str = path.to_string_lossy().to_string();
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--log-file", &path_str]),
        quiet_printer(),
    )
    .await
    .unwrap();

    send_one(r.smtp_addr.port(), "logged-1").await;
    send_one(r.smtp_addr.port(), "logged-2").await;

    for _ in 0..100 {
        if let Ok(s) = tokio::fs::read_to_string(&path).await {
            if s.lines().count() == 2 {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let body = tokio::fs::read_to_string(&path).await.unwrap();
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
    let _ = std::fs::remove_file(&path);
    r.shutdown();
}

#[tokio::test]
async fn relay_task_forwards_to_stub_upstream() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Stub upstream SMTP server.
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream.local_addr().unwrap();
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = upstream.accept().await else {
                return;
            };
            let received = received_clone.clone();
            tokio::spawn(async move {
                let (r, mut w) = sock.into_split();
                let mut r = BufReader::new(r);
                let _ = w.write_all(b"220 stub ready\r\n").await;
                // EHLO
                let mut line = String::new();
                let _ = r.read_line(&mut line).await;
                let _ = w.write_all(b"250-stub\r\n250 OK\r\n").await;
                // MAIL
                line.clear();
                let _ = r.read_line(&mut line).await;
                let _ = w.write_all(b"250 ok sender\r\n").await;
                // RCPT
                line.clear();
                let _ = r.read_line(&mut line).await;
                let _ = w.write_all(b"250 ok rcpt\r\n").await;
                // DATA
                line.clear();
                let _ = r.read_line(&mut line).await;
                let _ = w.write_all(b"354 go ahead\r\n").await;
                // Body until "\r\n.\r\n"
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
                // QUIT
                line.clear();
                let _ = r.read_line(&mut line).await;
                let _ = w.write_all(b"221 bye\r\n").await;
            });
        }
    });

    let url = format!("smtp://{}", upstream_addr);
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--relay", &url]),
        quiet_printer(),
    )
    .await
    .unwrap();
    send_one(r.smtp_addr.port(), "relayed").await;

    for _ in 0..200 {
        if !received.lock().await.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let got = received.lock().await.clone();
    assert!(!got.is_empty(), "upstream did not receive the relayed body");
    assert!(got[0].contains("Subject: relayed"), "{got:?}");
    r.shutdown();
}

#[tokio::test]
async fn buffer_size_evicts_oldest_messages() {
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--buffer-size", "3"]),
        quiet_printer(),
    )
    .await
    .unwrap();
    for s in ["a", "b", "c", "d", "e"] {
        send_one(r.smtp_addr.port(), s).await;
    }
    for _ in 0..50 {
        if r.store.len() == 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let subjects: Vec<String> = r
        .store
        .list(10)
        .into_iter()
        .map(|m| m.subject.unwrap_or_default())
        .collect();
    assert_eq!(
        subjects,
        vec!["e".to_string(), "d".to_string(), "c".to_string()]
    );
    r.shutdown();
}
