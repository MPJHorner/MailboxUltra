//! Bridge between the message store's broadcast channel and egui's repaint
//! signal. The egui main loop sleeps when no input arrives; new mail is not
//! input, so without this bridge the inbox would freeze until the user
//! moved the mouse.
//!
//! We re-subscribe automatically when the underlying store is replaced (the
//! ServerHandle::restart() path swaps the `Arc<MessageStore>` for a fresh
//! one when the buffer size or any SMTP-affecting setting changes).

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::server::ServerHandle;
use crate::store::MessageStore;

pub struct StoreSubscription {
    server: Arc<ServerHandle>,
    rt: Handle,
    egui_ctx: egui::Context,
    last_store: Option<Arc<MessageStore>>,
    task: Option<JoinHandle<()>>,
}

impl StoreSubscription {
    pub fn new(server: Arc<ServerHandle>, rt: Handle, egui_ctx: egui::Context) -> Self {
        let mut s = Self {
            server,
            rt,
            egui_ctx,
            last_store: None,
            task: None,
        };
        s.refresh();
        s
    }

    /// Cheap pointer-equality check; spins up a new bridge task only when
    /// the underlying store has actually been swapped out.
    pub fn refresh(&mut self) {
        let current = self.server.store();
        if let Some(last) = &self.last_store {
            if Arc::ptr_eq(last, &current) {
                return;
            }
        }
        if let Some(t) = self.task.take() {
            t.abort();
        }
        let mut rx = current.subscribe();
        let ctx = self.egui_ctx.clone();
        let _guard = self.rt.enter();
        self.task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_) => ctx.request_repaint(),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // We dropped some events but the store state will
                        // catch up on the next list() call; force a paint so
                        // the user sees the update.
                        ctx.request_repaint();
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
        self.last_store = Some(current);
    }
}

impl Drop for StoreSubscription {
    fn drop(&mut self) {
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}
