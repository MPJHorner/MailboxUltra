//! End-to-end SMTP tests. Drive the running server with `lettre`, the most
//! popular Rust SMTP client, so we know our server speaks the same dialect
//! production code does.

use std::time::Duration;

use clap::Parser;
use lettre::{
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::{authentication::Credentials, client::Tls},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use mailbox_ultra::{
    app,
    cli::Cli,
    output::{Printer, PrinterOptions},
};

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

fn transport(port: u16) -> AsyncSmtpTransport<Tokio1Executor> {
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
        .port(port)
        .tls(Tls::None)
        .timeout(Some(Duration::from_secs(5)))
        .build()
}

#[tokio::test]
async fn lettre_can_send_a_simple_message() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let port = r.smtp_addr.port();
    let mailer = transport(port);

    let email = Message::builder()
        .from("Alice <alice@example.com>".parse().unwrap())
        .to("bob@example.com".parse().unwrap())
        .subject("hi from lettre")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("hello world"))
        .unwrap();
    mailer.send(email).await.unwrap();

    // Give the server a moment to push to the store.
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let msgs = r.store.list(10);
    assert_eq!(msgs.len(), 1);
    let m = &msgs[0];
    assert_eq!(m.subject.as_deref(), Some("hi from lettre"));
    assert_eq!(m.envelope_from, "alice@example.com");
    assert_eq!(m.envelope_to, vec!["bob@example.com".to_string()]);
    assert!(m.text.as_deref().unwrap().contains("hello world"));
    r.shutdown();
}

#[tokio::test]
async fn lettre_html_and_text_alternative() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let mailer = transport(r.smtp_addr.port());

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
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let m = &r.store.list(10)[0];
    assert!(m.text.as_deref().unwrap().contains("plain body"));
    assert!(m.html.as_deref().unwrap().contains("html body"));
    r.shutdown();
}

#[tokio::test]
async fn lettre_attachment_preserved() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let mailer = transport(r.smtp_addr.port());

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
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let m = &r.store.list(10)[0];
    assert_eq!(m.attachments.len(), 1);
    assert_eq!(m.attachments[0].filename.as_deref(), Some("file.txt"));
    r.shutdown();
}

#[tokio::test]
async fn auth_required_rejects_unauthenticated() {
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--auth", "alice:s3cret"]),
        quiet_printer(),
    )
    .await
    .unwrap();
    let mailer = transport(r.smtp_addr.port());
    let email = Message::builder()
        .from("a@x".parse().unwrap())
        .to("b@x".parse().unwrap())
        .subject("nope")
        .body(String::from("body"))
        .unwrap();
    let res = mailer.send(email).await;
    assert!(res.is_err(), "expected auth error");
    assert_eq!(r.store.len(), 0);
    r.shutdown();
}

#[tokio::test]
async fn auth_with_correct_credentials_accepted() {
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--auth", "alice:s3cret"]),
        quiet_printer(),
    )
    .await
    .unwrap();
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
        .port(r.smtp_addr.port())
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
    mailer.send(email).await.unwrap();
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let msgs = r.store.list(10);
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].authenticated);
    r.shutdown();
}

#[tokio::test]
async fn message_too_large_rejected() {
    let r = app::start(
        &cli(&["-s", "0", "-u", "0", "--max-message-size", "256"]),
        quiet_printer(),
    )
    .await
    .unwrap();
    let mailer = transport(r.smtp_addr.port());
    let big = "X".repeat(2000);
    let email = Message::builder()
        .from("a@x".parse().unwrap())
        .to("b@x".parse().unwrap())
        .subject("big")
        .body(big)
        .unwrap();
    let res = mailer.send(email).await;
    assert!(res.is_err(), "expected oversize rejection");
    assert_eq!(r.store.len(), 0);
    r.shutdown();
}
