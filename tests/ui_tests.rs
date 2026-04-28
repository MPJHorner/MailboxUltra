//! HTTP UI / API integration tests. The pattern: start the servers, send a
//! message via lettre, then poke the JSON API + SSE stream.

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
        .from("Alice <alice@example.com>".parse().unwrap())
        .to("bob@example.com".parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(format!("body for {subject}"))
        .unwrap();
    mailer.send(email).await.unwrap();
}

#[tokio::test]
async fn list_endpoint_returns_captured_messages() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    send_one(r.smtp_addr.port(), "first").await;
    send_one(r.smtp_addr.port(), "second").await;
    for _ in 0..50 {
        if r.store.len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let ui = r.ui_addr.unwrap();
    let messages: serde_json::Value = reqwest::get(format!("http://{ui}/api/messages"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = messages.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // newest first
    assert_eq!(arr[0]["subject"], "second");
    assert_eq!(arr[1]["subject"], "first");
    r.shutdown();
}

#[tokio::test]
async fn get_message_by_id_and_raw() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    send_one(r.smtp_addr.port(), "wave").await;
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let id = r.store.list(1)[0].id;
    let ui = r.ui_addr.unwrap();
    let one: serde_json::Value = reqwest::get(format!("http://{ui}/api/messages/{id}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(one["subject"], "wave");

    let raw_resp = reqwest::get(format!("http://{ui}/api/messages/{id}/raw"))
        .await
        .unwrap();
    assert_eq!(
        raw_resp.headers().get("content-type").unwrap(),
        "message/rfc822"
    );
    let raw_body = raw_resp.text().await.unwrap();
    assert!(raw_body.contains("Subject: wave"));
    r.shutdown();
}

#[tokio::test]
async fn delete_and_clear_endpoints() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    for s in ["a", "b", "c"] {
        send_one(r.smtp_addr.port(), s).await;
    }
    for _ in 0..50 {
        if r.store.len() == 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let ui = r.ui_addr.unwrap();
    let id = r.store.list(10)[0].id;
    let resp = reqwest::Client::new()
        .delete(format!("http://{ui}/api/messages/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
    assert_eq!(r.store.len(), 2);

    let resp = reqwest::Client::new()
        .delete(format!("http://{ui}/api/messages"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
    assert!(r.store.is_empty());
    r.shutdown();
}

#[tokio::test]
async fn delete_unknown_id_returns_404() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let id = uuid::Uuid::new_v4();
    let resp = reqwest::Client::new()
        .delete(format!("http://{ui}/api/messages/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    r.shutdown();
}

#[tokio::test]
async fn health_includes_smtp_port_and_count() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    send_one(r.smtp_addr.port(), "h").await;
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let ui = r.ui_addr.unwrap();
    let h: serde_json::Value = reqwest::get(format!("http://{ui}/api/health"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(h["status"], "ok");
    assert_eq!(h["count"], 1);
    assert_eq!(h["smtp_port"], r.smtp_addr.port());
    r.shutdown();
}

#[tokio::test]
async fn relay_get_put_delete_lifecycle() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let client = reqwest::Client::new();

    let v: serde_json::Value = client
        .get(format!("http://{ui}/api/relay"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["enabled"], false);

    let v: serde_json::Value = client
        .put(format!("http://{ui}/api/relay"))
        .json(&serde_json::json!({"url": "smtp://relay.example.com:25"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["enabled"], true);
    assert_eq!(v["host"], "relay.example.com");

    let resp = client
        .delete(format!("http://{ui}/api/relay"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let v: serde_json::Value = client
        .get(format!("http://{ui}/api/relay"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["enabled"], false);
    r.shutdown();
}

#[tokio::test]
async fn relay_put_rejects_invalid_url() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let resp = reqwest::Client::new()
        .put(format!("http://{ui}/api/relay"))
        .json(&serde_json::json!({"url": "http://not-smtp"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert!(v["error"].as_str().unwrap().contains("scheme"));
    r.shutdown();
}

#[tokio::test]
async fn root_serves_embedded_index() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let body = reqwest::get(format!("http://{ui}/"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("MailBox"));
    assert!(body.contains("Ultra"));
    r.shutdown();
}

#[tokio::test]
async fn unknown_static_path_returns_404() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let resp = reqwest::get(format!("http://{ui}/no-such-asset.txt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    r.shutdown();
}

#[tokio::test]
async fn sse_stream_pushes_new_messages() {
    use eventsource_client::{Client as _, ClientBuilder, SSE};
    use futures::stream::StreamExt;

    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    let ui = r.ui_addr.unwrap();
    let client = ClientBuilder::for_url(&format!("http://{ui}/api/stream"))
        .unwrap()
        .build();
    let mut stream = client.stream();

    // Wait for the hello event.
    let _hello = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .expect("no hello event")
        .unwrap()
        .unwrap();

    // Send a message and read the next event.
    send_one(r.smtp_addr.port(), "sse-test").await;

    let mut got = false;
    for _ in 0..50 {
        let next = tokio::time::timeout(Duration::from_secs(3), stream.next()).await;
        if let Ok(Some(Ok(SSE::Event(ev)))) = next {
            if ev.event_type == "message" && ev.data.contains("sse-test") {
                got = true;
                break;
            }
        }
    }
    assert!(got, "did not receive a 'message' SSE event for sse-test");
    r.shutdown();
}

#[tokio::test]
async fn release_endpoint_returns_400_on_bad_url() {
    let r = app::start(&cli(&["-s", "0", "-u", "0"]), quiet_printer())
        .await
        .unwrap();
    send_one(r.smtp_addr.port(), "rel").await;
    for _ in 0..50 {
        if r.store.len() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let id = r.store.list(1)[0].id;
    let ui = r.ui_addr.unwrap();
    let resp = reqwest::Client::new()
        .post(format!("http://{ui}/api/messages/{id}/release"))
        .json(&serde_json::json!({"smtp_url": "not-a-url"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    r.shutdown();
}
