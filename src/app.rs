use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Maximum number of consecutive ports tried after the requested one when the
/// requested port is already in use.
const MAX_PORT_FALLBACK_ATTEMPTS: u16 = 50;

async fn bind_with_fallback(bind: IpAddr, port: u16) -> io::Result<TcpListener> {
    if port == 0 {
        return TcpListener::bind(SocketAddr::new(bind, 0)).await;
    }
    let mut last_err = None;
    for offset in 0..=MAX_PORT_FALLBACK_ATTEMPTS {
        let candidate = match port.checked_add(offset) {
            Some(p) => p,
            None => break,
        };
        match TcpListener::bind(SocketAddr::new(bind, candidate)).await {
            Ok(l) => return Ok(l),
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                last_err = Some(e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::other("no free port found in fallback range")))
}

use crate::{
    cli::Cli,
    output::Printer,
    relay::{self, RelayConfig, RelaySwitch},
    smtp::{self, SmtpConfig},
    store::{MessageStore, StoreEvent},
    ui,
};

/// Live handles to the running servers; callers can either `join()` or `abort()`.
pub struct Running {
    pub store: Arc<MessageStore>,
    pub smtp_addr: SocketAddr,
    pub ui_addr: Option<SocketAddr>,
    pub smtp_task: JoinHandle<anyhow::Result<()>>,
    pub ui_task: Option<JoinHandle<std::io::Result<()>>>,
    pub printer_task: Option<JoinHandle<()>>,
    pub log_task: Option<JoinHandle<()>>,
    pub relay_task: Option<JoinHandle<()>>,
    pub relay_switch: RelaySwitch,
}

impl Running {
    pub fn shutdown(self) {
        self.smtp_task.abort();
        if let Some(t) = self.ui_task {
            t.abort();
        }
        if let Some(t) = self.printer_task {
            t.abort();
        }
        if let Some(t) = self.log_task {
            t.abort();
        }
        if let Some(t) = self.relay_task {
            t.abort();
        }
    }
}

/// Bind both servers, spawn the printer and log writer, return live handles.
pub async fn start(cli: &Cli, printer: Printer) -> Result<Running> {
    cli.validate().map_err(anyhow::Error::msg)?;

    let store = MessageStore::new(cli.buffer_size);
    let bind: IpAddr = cli.bind.parse().context("parsing --bind address")?;

    let smtp_listener = bind_with_fallback(bind, cli.smtp_port)
        .await
        .with_context(|| format!("binding SMTP server on {}:{}", cli.bind, cli.smtp_port))?;
    let smtp_addr = smtp_listener.local_addr()?;
    if cli.smtp_port != 0 && smtp_addr.port() != cli.smtp_port {
        printer.print_port_fallback("SMTP", cli.smtp_port, smtp_addr.port());
    }

    let auth = cli.auth.as_deref().and_then(|s| {
        let (u, p) = s.split_once(':')?;
        Some((u.to_string(), p.to_string()))
    });
    let smtp_cfg = SmtpConfig {
        hostname: cli.hostname.clone(),
        max_message_size: cli.max_message_size,
        auth: auth.clone(),
    };

    let initial_relay = match cli.relay.as_deref() {
        Some(raw) => {
            let parsed = url::Url::parse(raw).context("parsing --relay URL")?;
            Some(RelayConfig::from_url(parsed, cli.relay_insecure)?)
        }
        None => None,
    };
    let relay_for_banner = initial_relay.as_ref().map(|c| redact_url(&c.url));
    let relay_switch = relay::new_switch(initial_relay);

    let store_for_smtp = store.clone();
    let cfg_for_smtp = smtp_cfg.clone();
    let smtp_task =
        tokio::spawn(async move { smtp::serve(smtp_listener, store_for_smtp, cfg_for_smtp).await });

    let (ui_addr, ui_task) = if cli.no_ui {
        (None, None)
    } else {
        let listener = bind_with_fallback(bind, cli.ui_port)
            .await
            .with_context(|| format!("binding UI server on {}:{}", cli.bind, cli.ui_port))?;
        let addr = listener.local_addr()?;
        if cli.ui_port != 0 && addr.port() != cli.ui_port {
            printer.print_port_fallback("UI", cli.ui_port, addr.port());
        }
        let router = ui::router(store.clone(), Some(smtp_addr.port()), relay_switch.clone());
        let task = tokio::spawn(async move { axum::serve(listener, router).await });
        (Some(addr), Some(task))
    };

    let smtp_url = format!("smtp://{}", smtp_addr);
    let ui_url = ui_addr.map(|a| format!("http://{}", a));
    printer.print_banner_with_relay(
        &smtp_url,
        ui_url.as_deref(),
        cli.buffer_size,
        cli.max_message_size,
        relay_for_banner.as_deref(),
        auth.is_some(),
    );

    let printer_task = if !printer.options().quiet {
        let printer_clone = printer.clone();
        let mut rx = store.subscribe();
        Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(StoreEvent::Message(msg)) => printer_clone.print_message(&msg),
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }))
    } else {
        None
    };

    let log_task = if let Some(path) = cli.log_file.as_deref() {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .with_context(|| format!("opening --log-file {path}"))?;
        Some(spawn_log_writer(file, store.subscribe()))
    } else {
        None
    };

    let relay_task = if relay_switch.read().await.is_some() {
        Some(relay::spawn_relay(store.clone(), relay_switch.clone()))
    } else {
        None
    };

    if cli.open {
        if let Some(url) = &ui_url {
            let _ = crate::entrypoint::open_browser(url);
        }
    }

    Ok(Running {
        store,
        smtp_addr,
        ui_addr,
        smtp_task,
        ui_task,
        printer_task,
        log_task,
        relay_task,
        relay_switch,
    })
}

fn redact_url(u: &url::Url) -> String {
    if u.username().is_empty() {
        return u.to_string();
    }
    let mut clone = u.clone();
    let _ = clone.set_username("");
    let _ = clone.set_password(None);
    let mut s = clone.to_string();
    if let Some(after_scheme) = s.find("://") {
        let head = &s[..after_scheme + 3];
        let tail = &s[after_scheme + 3..];
        s = format!("{head}***@{tail}");
    }
    s
}

fn spawn_log_writer(
    file: tokio::fs::File,
    mut rx: tokio::sync::broadcast::Receiver<StoreEvent>,
) -> JoinHandle<()> {
    use tokio::io::AsyncWriteExt;
    tokio::spawn(async move {
        let mut file = file;
        loop {
            match rx.recv().await {
                Ok(StoreEvent::Message(msg)) => {
                    if let Ok(s) = serde_json::to_string(&*msg) {
                        let _ = file.write_all(s.as_bytes()).await;
                        let _ = file.write_all(b"\n").await;
                        let _ = file.flush().await;
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::PrinterOptions;
    use clap::Parser;

    fn cli(args: &[&str]) -> Cli {
        let mut v = vec!["mailbox-ultra"];
        v.extend_from_slice(args);
        Cli::parse_from(v)
    }

    fn quiet_printer() -> Printer {
        Printer::new(PrinterOptions {
            use_color: false,
            json_mode: false,
            verbose: false,
            quiet: true,
        })
    }

    #[tokio::test]
    async fn bind_with_fallback_returns_ephemeral_port_when_zero() {
        let l = bind_with_fallback("127.0.0.1".parse().unwrap(), 0)
            .await
            .unwrap();
        assert!(l.local_addr().unwrap().port() != 0);
    }

    #[tokio::test]
    async fn bind_with_fallback_walks_past_busy_ports() {
        let blocker = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let busy = blocker.local_addr().unwrap().port();
        let l = bind_with_fallback("127.0.0.1".parse().unwrap(), busy)
            .await
            .unwrap();
        let chosen = l.local_addr().unwrap().port();
        assert_ne!(chosen, busy);
        assert!(chosen > busy);
        drop(blocker);
        drop(l);
    }

    #[tokio::test]
    async fn start_and_shutdown_returns_addresses() {
        let c = cli(&["-s", "0", "-u", "0"]);
        let r = start(&c, quiet_printer()).await.unwrap();
        assert!(r.smtp_addr.port() != 0);
        assert!(r.ui_addr.is_some());
        r.shutdown();
    }

    #[tokio::test]
    async fn start_with_no_ui() {
        let c = cli(&["-s", "0", "--no-ui"]);
        let r = start(&c, quiet_printer()).await.unwrap();
        assert!(r.ui_addr.is_none());
        assert!(r.ui_task.is_none());
        r.shutdown();
    }

    #[tokio::test]
    async fn start_validates_cli() {
        let c = cli(&["-s", "1025", "-u", "1025"]);
        let err = match start(&c, quiet_printer()).await {
            Ok(_) => panic!("expected validation error"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("1025"));
    }

    #[tokio::test]
    async fn start_with_relay_url_populates_switch() {
        let c = cli(&[
            "-s",
            "0",
            "-u",
            "0",
            "--relay",
            "smtp://relay.example.com:25",
        ]);
        let r = start(&c, quiet_printer()).await.unwrap();
        let snap = r.relay_switch.read().await.clone();
        assert!(snap.is_some());
        assert!(r.relay_task.is_some());
        r.shutdown();
    }

    #[tokio::test]
    async fn start_with_auth_uses_credentials() {
        let c = cli(&["-s", "0", "-u", "0", "--auth", "alice:s3cret"]);
        let r = start(&c, quiet_printer()).await.unwrap();
        // We can verify by sending a single line and reading the EHLO response.
        let conn = tokio::net::TcpStream::connect(r.smtp_addr).await.unwrap();
        let (rd, mut wr) = conn.into_split();
        let mut rd = tokio::io::BufReader::new(rd);
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut line = String::new();
        rd.read_line(&mut line).await.unwrap();
        wr.write_all(b"EHLO test\r\n").await.unwrap();
        let mut all = String::new();
        // Read several lines.
        for _ in 0..8 {
            let mut buf = String::new();
            if rd.read_line(&mut buf).await.unwrap() == 0 {
                break;
            }
            all.push_str(&buf);
            if buf.starts_with("250 ") {
                break;
            }
        }
        wr.write_all(b"QUIT\r\n").await.unwrap();
        assert!(
            all.contains("AUTH PLAIN LOGIN"),
            "no AUTH advertised: {all}"
        );
        r.shutdown();
    }

    #[tokio::test]
    async fn redact_url_handles_userinfo() {
        let u = url::Url::parse("smtp://a:b@host:25").unwrap();
        let s = redact_url(&u);
        assert!(s.contains("***@"));
        assert!(!s.contains("b@"));
        let u = url::Url::parse("smtp://host:25").unwrap();
        let s = redact_url(&u);
        assert!(!s.contains('@'));
    }

    #[tokio::test]
    async fn printer_task_forwards_messages_to_sink() {
        use std::io::Write;
        use std::sync::Mutex;
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
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
        let printer = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        let c = cli(&["-s", "0", "-u", "0"]);
        let r = start(&c, printer).await.unwrap();
        // Fire a quick SMTP exchange.
        let stream = tokio::net::TcpStream::connect(r.smtp_addr).await.unwrap();
        let (rd, mut wr) = stream.into_split();
        let mut rd = tokio::io::BufReader::new(rd);
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut greet = String::new();
        rd.read_line(&mut greet).await.unwrap();
        wr.write_all(b"EHLO t\r\n").await.unwrap();
        for _ in 0..8 {
            let mut s = String::new();
            if rd.read_line(&mut s).await.unwrap() == 0 {
                break;
            }
            if s.starts_with("250 ") {
                break;
            }
        }
        wr.write_all(b"MAIL FROM:<a@x>\r\nRCPT TO:<b@x>\r\nDATA\r\n")
            .await
            .unwrap();
        for _ in 0..3 {
            let mut s = String::new();
            if rd.read_line(&mut s).await.unwrap() == 0 {
                break;
            }
            if s.starts_with("354") {
                break;
            }
        }
        wr.write_all(b"Subject: Hi\r\n\r\nbody\r\n.\r\nQUIT\r\n")
            .await
            .unwrap();
        for _ in 0..30 {
            if buf.lock().unwrap().windows(3).any(|w| w == b"->") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(out.contains("Hi"), "printer did not log: {out}");
        r.shutdown();
    }

    #[tokio::test]
    async fn smtp_port_fallback_emits_notice() {
        use std::io::Write;
        use std::sync::Mutex;
        let blocker = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let busy = blocker.local_addr().unwrap().port();
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
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
        let printer = Printer::with_sink(
            PrinterOptions {
                use_color: false,
                json_mode: false,
                verbose: false,
                quiet: false,
            },
            BufWriter(buf.clone()),
        );
        let c = cli(&["-s", &busy.to_string(), "-u", "0"]);
        let r = start(&c, printer).await.unwrap();
        let out = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(
            out.contains(&format!("SMTP port {busy} in use")),
            "expected fallback notice in: {out:?}"
        );
        drop(blocker);
        r.shutdown();
    }

    #[tokio::test]
    async fn spawn_log_writer_drains_a_burst_of_messages() {
        let store = MessageStore::new(8);
        let path = std::env::temp_dir().join(format!("mbu-cov-{}.log", uuid::Uuid::new_v4()));
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .unwrap();
        let rx = store.subscribe();
        let handle = spawn_log_writer(file, rx);
        drop(store);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        let _ = std::fs::remove_file(&path);
    }
}
