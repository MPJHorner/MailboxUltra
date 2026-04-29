//! Server lifecycle: bind the SMTP listener, spawn the relay + optional log
//! writer, expose snapshot accessors for the GUI, and support a restart flow
//! when settings change.
//!
//! `ServerHandle` is the only piece of code in the app that talks to tokio
//! directly. The GUI (egui, sync) holds an `Arc<ServerHandle>`, calls the
//! snapshot accessors every frame to read store contents, and calls
//! `restart()` when the user clicks Apply in Preferences.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::message::Message;
use crate::relay::{self, RelayConfig, RelaySwitch};
use crate::settings::PersistentSettings;
use crate::smtp::{self, SmtpConfig};
use crate::store::{MessageStore, StoreEvent};

/// Maximum number of consecutive ports tried after the requested one when the
/// requested port is already in use. Mirrors the old CLI behaviour.
const MAX_PORT_FALLBACK_ATTEMPTS: u16 = 50;

/// What changed during the most recent `restart()`. The GUI uses this to
/// decide which toast to show.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RestartReport {
    /// `Some(addr)` when the SMTP listener was rebound. The address may
    /// differ from the requested one if the port fell back.
    pub smtp_restarted: Option<SocketAddr>,
    /// True when the log file changed (started, stopped, or moved).
    pub log_changed: bool,
    /// True when the relay configuration changed (URL, insecure, or on/off).
    pub relay_changed: bool,
    /// The captured messages preserved across an SMTP restart.
    pub messages_preserved: usize,
}

impl RestartReport {
    pub fn nothing_changed(&self) -> bool {
        self.smtp_restarted.is_none() && !self.log_changed && !self.relay_changed
    }
}

/// Lifecycle handle for all background work. Cheap to clone (it's `Arc`d
/// internally).
pub struct ServerHandle {
    inner: RwLock<Inner>,
    rt: Handle,
}

struct Inner {
    settings: PersistentSettings,
    store: Arc<MessageStore>,
    smtp_addr: SocketAddr,
    smtp_task: JoinHandle<anyhow::Result<()>>,
    log_task: Option<JoinHandle<()>>,
    relay_task: JoinHandle<()>,
    relay_switch: RelaySwitch,
}

impl ServerHandle {
    /// Bring up the servers using `settings`. Returns once the SMTP listener
    /// is bound and ready (so the GUI can show the bound address immediately).
    pub fn start(rt: Handle, settings: PersistentSettings) -> Result<Arc<Self>> {
        let inner = build_inner(&rt, settings)?;
        Ok(Arc::new(Self {
            inner: RwLock::new(inner),
            rt,
        }))
    }

    /// Apply `new` to the running servers. Restart strategy:
    ///
    /// * Hot-update only — relay url / insecure flag (writes to the existing
    ///   `RelaySwitch`).
    /// * Log-task only — log file changed but nothing else did.
    /// * Full restart — anything that affects the SMTP listener or the
    ///   message store. Messages already in the store are copied into the
    ///   replacement store (up to the new buffer cap) so the user does not
    ///   lose their captured inbox on a port change.
    pub fn restart(&self, new: PersistentSettings) -> Result<RestartReport> {
        // Validate the new relay URL first so we can fail without disturbing
        // the running servers. `build_relay_config` returns an error for a
        // malformed URL or unsupported scheme.
        let new_relay = build_relay_config(&new)?;

        let mut inner = self.inner.write().expect("server inner poisoned");
        let old = inner.settings.clone();

        let smtp_or_store_changed = new.smtp_port != old.smtp_port
            || new.bind != old.bind
            || new.hostname != old.hostname
            || new.max_message_size != old.max_message_size
            || new.auth != old.auth
            || new.buffer_size != old.buffer_size;

        let mut report = RestartReport::default();

        if smtp_or_store_changed {
            let preserved = inner.store.list(new.buffer_size);
            let preserved_count = preserved.len();

            inner.smtp_task.abort();
            if let Some(t) = inner.log_task.take() {
                t.abort();
            }
            inner.relay_task.abort();

            let new_store = MessageStore::new(new.buffer_size);
            // `list` returns newest-first, so we replay oldest-first to keep
            // the original chronological order in the new store.
            for msg in preserved.into_iter().rev() {
                new_store.push(msg);
            }

            let listener = self
                .rt
                .block_on(bind_with_fallback(new.bind, new.smtp_port))
                .with_context(|| format!("rebinding SMTP on {}:{}", new.bind, new.smtp_port))?;
            let smtp_addr = listener.local_addr()?;

            // `relay::spawn_relay` uses bare `tokio::spawn`; we need an enter
            // guard for the synchronous spawn calls below.
            let _guard = self.rt.enter();
            let cfg = build_smtp_config(&new);
            let smtp_store = new_store.clone();
            let smtp_task = self
                .rt
                .spawn(async move { smtp::serve(listener, smtp_store, cfg).await });

            let relay_task = relay::spawn_relay(new_store.clone(), inner.relay_switch.clone());

            let log_task = match &new.log_file {
                Some(path) => Some(spawn_log_task(&self.rt, path, new_store.subscribe())?),
                None => None,
            };

            inner.store = new_store;
            inner.smtp_addr = smtp_addr;
            inner.smtp_task = smtp_task;
            inner.relay_task = relay_task;
            inner.log_task = log_task;

            report.smtp_restarted = Some(smtp_addr);
            report.messages_preserved = preserved_count;
        } else if new.log_file != old.log_file {
            if let Some(t) = inner.log_task.take() {
                t.abort();
            }
            if let Some(path) = &new.log_file {
                inner.log_task = Some(spawn_log_task(&self.rt, path, inner.store.subscribe())?);
            }
            report.log_changed = true;
        }

        if new.relay != old.relay {
            let switch = inner.relay_switch.clone();
            self.rt.block_on(async move {
                *switch.write().await = new_relay;
            });
            report.relay_changed = true;
        }

        inner.settings = new;
        Ok(report)
    }

    /// Best-effort task abort. Tokio drops outstanding tasks when the runtime
    /// is dropped at process exit, so calling this is optional in practice.
    pub fn shutdown(&self) {
        let mut inner = self.inner.write().expect("server inner poisoned");
        inner.smtp_task.abort();
        if let Some(t) = inner.log_task.take() {
            t.abort();
        }
        inner.relay_task.abort();
    }

    pub fn store(&self) -> Arc<MessageStore> {
        self.inner
            .read()
            .expect("server inner poisoned")
            .store
            .clone()
    }

    pub fn smtp_addr(&self) -> SocketAddr {
        self.inner.read().expect("server inner poisoned").smtp_addr
    }

    pub fn relay_switch(&self) -> RelaySwitch {
        self.inner
            .read()
            .expect("server inner poisoned")
            .relay_switch
            .clone()
    }

    pub fn settings(&self) -> PersistentSettings {
        self.inner
            .read()
            .expect("server inner poisoned")
            .settings
            .clone()
    }
}

fn build_inner(rt: &Handle, settings: PersistentSettings) -> Result<Inner> {
    // `relay::spawn_relay` uses bare `tokio::spawn`, which requires an active
    // runtime context. We're called from synchronous code (the GUI thread or
    // the test runtime), so we enter the runtime here for the duration of the
    // spawn calls.
    let _guard = rt.enter();

    let store = MessageStore::new(settings.buffer_size);

    let listener = rt
        .block_on(bind_with_fallback(settings.bind, settings.smtp_port))
        .with_context(|| {
            format!(
                "binding SMTP server on {}:{}",
                settings.bind, settings.smtp_port
            )
        })?;
    let smtp_addr = listener.local_addr()?;

    let cfg = build_smtp_config(&settings);
    let smtp_store = store.clone();
    let smtp_task = rt.spawn(async move { smtp::serve(listener, smtp_store, cfg).await });

    let initial_relay = build_relay_config(&settings)?;
    let relay_switch = relay::new_switch(initial_relay);
    let relay_task = relay::spawn_relay(store.clone(), relay_switch.clone());

    let log_task = match &settings.log_file {
        Some(path) => Some(spawn_log_task(rt, path, store.subscribe())?),
        None => None,
    };

    Ok(Inner {
        settings,
        store,
        smtp_addr,
        smtp_task,
        log_task,
        relay_task,
        relay_switch,
    })
}

fn build_smtp_config(s: &PersistentSettings) -> SmtpConfig {
    SmtpConfig {
        hostname: s.hostname.clone(),
        max_message_size: s.max_message_size,
        auth: s.auth.as_ref().map(|a| (a.user.clone(), a.pass.clone())),
    }
}

fn build_relay_config(s: &PersistentSettings) -> Result<Option<RelayConfig>> {
    match &s.relay {
        None => Ok(None),
        Some(r) => {
            let parsed = url::Url::parse(&r.url)
                .with_context(|| format!("parsing relay URL '{}'", r.url))?;
            Ok(Some(RelayConfig::from_url(parsed, r.insecure)?))
        }
    }
}

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

fn spawn_log_task(
    rt: &Handle,
    path: &Path,
    rx: tokio::sync::broadcast::Receiver<StoreEvent>,
) -> Result<JoinHandle<()>> {
    let file = rt
        .block_on(async {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await
        })
        .with_context(|| format!("opening log file {}", path.display()))?;
    Ok(rt.spawn(log_writer_loop(file, rx)))
}

async fn log_writer_loop(
    file: tokio::fs::File,
    mut rx: tokio::sync::broadcast::Receiver<StoreEvent>,
) {
    use tokio::io::AsyncWriteExt;
    let mut file = file;
    loop {
        match rx.recv().await {
            Ok(StoreEvent::Message(msg)) => {
                let line: &Message = &msg;
                if let Ok(s) = serde_json::to_string(line) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Auth, RelaySettings};

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn ephemeral_settings() -> PersistentSettings {
        PersistentSettings {
            smtp_port: 0,
            buffer_size: 16,
            ..PersistentSettings::default()
        }
    }

    #[test]
    fn start_binds_an_ephemeral_smtp_port() {
        let rt = rt();
        let h = ServerHandle::start(rt.handle().clone(), ephemeral_settings()).unwrap();
        assert!(h.smtp_addr().port() != 0);
        assert_eq!(h.store().len(), 0);
        h.shutdown();
    }

    #[test]
    fn restart_with_no_changes_is_a_noop() {
        let rt = rt();
        let h = ServerHandle::start(rt.handle().clone(), ephemeral_settings()).unwrap();
        let same = h.settings();
        let report = h.restart(same).unwrap();
        assert!(report.nothing_changed(), "{report:?}");
        h.shutdown();
    }

    #[test]
    fn restart_changing_buffer_size_preserves_messages() {
        use bytes::Bytes;
        let rt = rt();
        let h = ServerHandle::start(rt.handle().clone(), ephemeral_settings()).unwrap();

        let store = h.store();
        for n in 0..5 {
            let raw = Bytes::copy_from_slice(
                format!("From: a@x\r\nTo: b@x\r\nSubject: m{n}\r\n\r\nbody {n}\r\n").as_bytes(),
            );
            let msg = crate::message::parse_message(
                raw,
                "a@x".into(),
                vec!["b@x".into()],
                "1.1.1.1:1".into(),
                false,
            );
            store.push(msg);
        }
        assert_eq!(store.len(), 5);

        let mut new_settings = h.settings();
        new_settings.buffer_size = 3;
        // capture the currently-bound port so the SMTP listener doesn't move
        new_settings.smtp_port = h.smtp_addr().port();
        let report = h.restart(new_settings).unwrap();

        assert!(report.smtp_restarted.is_some());
        assert_eq!(report.messages_preserved, 3);

        // Three most-recent messages survive, oldest-first chronological order.
        let preserved = h.store().list(10);
        let subjects: Vec<_> = preserved
            .iter()
            .map(|m| m.subject.clone().unwrap_or_default())
            .collect();
        assert_eq!(
            subjects,
            vec!["m4".to_string(), "m3".to_string(), "m2".to_string()]
        );
        h.shutdown();
    }

    #[test]
    fn restart_relay_only_uses_hot_update() {
        let rt = rt();
        let h = ServerHandle::start(rt.handle().clone(), ephemeral_settings()).unwrap();
        let port_before = h.smtp_addr().port();

        let mut new_settings = h.settings();
        new_settings.relay = Some(RelaySettings {
            url: "smtp://relay.example.com:25".into(),
            insecure: false,
        });
        let report = h.restart(new_settings).unwrap();

        assert!(report.relay_changed);
        assert!(report.smtp_restarted.is_none());
        assert_eq!(h.smtp_addr().port(), port_before);
        let snap = rt
            .handle()
            .block_on(async { h.relay_switch().read().await.clone() });
        assert!(snap.is_some());
        h.shutdown();
    }

    #[test]
    fn restart_invalid_relay_url_does_not_change_state() {
        let rt = rt();
        let h = ServerHandle::start(rt.handle().clone(), ephemeral_settings()).unwrap();
        let port_before = h.smtp_addr().port();

        let mut new_settings = h.settings();
        new_settings.relay = Some(RelaySettings {
            url: "::not-a-url::".into(),
            insecure: false,
        });
        let err = h.restart(new_settings).unwrap_err();
        assert!(err.to_string().contains("parsing relay URL"));
        // Server still up on the same port.
        assert_eq!(h.smtp_addr().port(), port_before);
        h.shutdown();
    }

    #[test]
    fn start_with_auth_advertises_it() {
        let rt = rt();
        let mut s = ephemeral_settings();
        s.auth = Some(Auth {
            user: "alice".into(),
            pass: "s3cret".into(),
        });
        let h = ServerHandle::start(rt.handle().clone(), s).unwrap();
        let addr = h.smtp_addr();
        let banner = rt.handle().block_on(async move {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (rd, mut wr) = stream.into_split();
            let mut rd = tokio::io::BufReader::new(rd);
            let mut greeting = String::new();
            rd.read_line(&mut greeting).await.unwrap();
            wr.write_all(b"EHLO test\r\n").await.unwrap();
            let mut all = String::new();
            for _ in 0..10 {
                let mut line = String::new();
                if rd.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                all.push_str(&line);
                if line.starts_with("250 ") {
                    break;
                }
            }
            all
        });
        assert!(banner.contains("AUTH PLAIN LOGIN"), "{banner}");
        h.shutdown();
    }

    #[test]
    fn build_relay_config_rejects_bad_scheme() {
        let mut s = ephemeral_settings();
        s.relay = Some(RelaySettings {
            url: "http://nope".into(),
            insecure: false,
        });
        let err = build_relay_config(&s).unwrap_err();
        assert!(err.to_string().contains("smtp or smtps"));
    }

    #[test]
    fn build_smtp_config_maps_auth_correctly() {
        let mut s = ephemeral_settings();
        s.auth = Some(Auth {
            user: "u".into(),
            pass: "p".into(),
        });
        let cfg = build_smtp_config(&s);
        assert_eq!(cfg.auth, Some(("u".into(), "p".into())));
        assert_eq!(cfg.hostname, s.hostname);
        assert_eq!(cfg.max_message_size, s.max_message_size);
    }
}
