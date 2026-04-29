//! Release tab — send the captured message to an upstream SMTP URL.
//!
//! The relay call is async (network I/O), so we spawn it on the tokio
//! runtime and track progress with a `oneshot` channel. Each frame we poll
//! the receiver; when it resolves we toast and clear the spinner.

use std::sync::Arc;

use egui::RichText;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::message::Message;
use crate::relay::{relay_message, RelayConfig};
use crate::server::ServerHandle;

use super::DetailContext;

#[derive(Default)]
pub struct ReleaseState {
    pub url: String,
    pub insecure: bool,
    in_flight: Option<InFlight>,
    last_error: Option<String>,
}

struct InFlight {
    message_id: Uuid,
    rx: oneshot::Receiver<anyhow::Result<()>>,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut ReleaseState,
    message: &Message,
    ctx: &mut DetailContext<'_>,
) {
    poll_in_flight(state, ctx);

    ui.label(
        RichText::new("Send this captured message to an upstream SMTP server.")
            .small()
            .color(ui.style().visuals.weak_text_color()),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("URL");
        ui.add(
            egui::TextEdit::singleline(&mut state.url)
                .hint_text("smtp://relay.example.com:25")
                .desired_width(360.0),
        );
    });
    ui.checkbox(
        &mut state.insecure,
        "Skip TLS certificate verification (smtps:// only)",
    );
    if let Some(err) = &state.last_error {
        ui.add_space(4.0);
        ui.label(
            RichText::new(err)
                .color(egui::Color32::from_rgb(248, 113, 113))
                .small(),
        );
    }
    ui.add_space(8.0);

    let in_flight = state
        .in_flight
        .as_ref()
        .map(|f| f.message_id == message.id)
        .unwrap_or(false);

    ui.horizontal(|ui| {
        if in_flight {
            ui.add(egui::Spinner::new());
            ui.label("Sending…");
        } else {
            let send = ui.add_enabled(!state.url.trim().is_empty(), egui::Button::new("Send"));
            if send.clicked() {
                start_send(state, message, ctx.server);
            }
        }
    });
}

fn start_send(state: &mut ReleaseState, message: &Message, server: &Arc<ServerHandle>) {
    state.last_error = None;
    let url = state.url.trim().to_string();
    let insecure = state.insecure;

    let parsed = match url::Url::parse(&url) {
        Ok(u) => u,
        Err(e) => {
            state.last_error = Some(format!("Invalid URL: {e}"));
            return;
        }
    };
    let cfg = match RelayConfig::from_url(parsed, insecure) {
        Ok(c) => c,
        Err(e) => {
            state.last_error = Some(format!("Invalid relay: {e}"));
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    let m = Box::new(message.clone());
    let _guard = tokio::runtime::Handle::current().enter();
    tokio::spawn(async move {
        let result = relay_message(&cfg, &m).await;
        let _ = tx.send(result);
    });
    state.in_flight = Some(InFlight {
        message_id: message.id,
        rx,
    });
    let _ = server; // future: integrate with server's relay switch when offered
}

fn poll_in_flight(state: &mut ReleaseState, ctx: &mut DetailContext<'_>) {
    let Some(mut f) = state.in_flight.take() else {
        return;
    };
    match f.rx.try_recv() {
        Ok(Ok(())) => {
            ctx.toasts.success("Released to upstream");
        }
        Ok(Err(e)) => {
            let msg = format!("Release failed: {e}");
            state.last_error = Some(msg.clone());
            ctx.toasts.error(msg);
        }
        Err(oneshot::error::TryRecvError::Empty) => {
            state.in_flight = Some(f);
        }
        Err(oneshot::error::TryRecvError::Closed) => {
            ctx.toasts
                .error("Release task aborted before completion");
        }
    }
}
