//! Relay configuration as its own small window, accessible from the
//! toolbar Relay button so users can toggle relay without opening
//! Preferences.

use std::sync::Arc;

use egui::RichText;

use crate::server::ServerHandle;
use crate::settings::{PersistentSettings, RelaySettings};

use super::theme;
use super::toasts::ToastList;
use super::widgets;

#[derive(Default)]
pub struct RelayWindowState {
    pub open: bool,
    pub url: String,
    pub insecure: bool,
    pub enabled: bool,
    pub last_error: Option<String>,
}

impl RelayWindowState {
    pub fn open_with(&mut self, current: &PersistentSettings) {
        self.open = true;
        self.last_error = None;
        match &current.relay {
            Some(r) => {
                self.url = r.url.clone();
                self.insecure = r.insecure;
                self.enabled = true;
            }
            None => {
                self.url.clear();
                self.insecure = false;
                self.enabled = false;
            }
        }
    }
    pub fn close(&mut self) {
        self.open = false;
        self.last_error = None;
    }
}

pub fn render(
    ctx: &egui::Context,
    state: &mut RelayWindowState,
    server: &Arc<ServerHandle>,
    toasts: &mut ToastList,
) {
    if !state.open {
        return;
    }
    let mut keep_open = true;
    let mut save_clicked = false;
    let mut disable_clicked = false;
    let mut cancel_clicked = false;

    egui::Window::new(RichText::new("Upstream relay").strong())
        .open(&mut keep_open)
        .resizable(false)
        .collapsible(false)
        .default_size([460.0, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Forward every captured message to a real upstream MTA after \
                    capture. Use smtp:// for plain delivery, smtps:// to wrap in TLS.",
                )
                .small()
                .color(theme::muted_text_color(ui.ctx())),
            );
            ui.add_space(10.0);
            widgets::nice_checkbox(ui, &mut state.enabled, "Forward to upstream");
            if state.enabled {
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.url)
                        .hint_text("smtp://relay.example.com:25")
                        .desired_width(420.0),
                );
                ui.add_space(4.0);
                widgets::nice_checkbox(
                    ui,
                    &mut state.insecure,
                    "Skip TLS certificate verification (dev only)",
                );
            }
            if let Some(err) = &state.last_error {
                ui.add_space(6.0);
                ui.label(RichText::new(err).color(theme::DANGER).small());
            }
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let save = egui::Button::new(
                        RichText::new("Save").color(egui::Color32::WHITE).strong(),
                    )
                    .fill(theme::accent(ui.ctx()))
                    .stroke(egui::Stroke::new(1.0, theme::accent(ui.ctx())));
                    if ui.add(save).clicked() {
                        save_clicked = true;
                    }
                    if ui
                        .add_enabled(state.enabled, egui::Button::new("Disable relay"))
                        .clicked()
                    {
                        disable_clicked = true;
                    }
                });
            });
        });

    if !keep_open || cancel_clicked {
        state.close();
        return;
    }
    if disable_clicked {
        let mut new_settings = server.settings();
        new_settings.relay = None;
        apply(state, server, toasts, new_settings);
        return;
    }
    if save_clicked {
        if !state.enabled {
            let mut new_settings = server.settings();
            new_settings.relay = None;
            apply(state, server, toasts, new_settings);
            return;
        }
        let url = state.url.trim();
        if url.is_empty() {
            state.last_error = Some("URL must not be empty".into());
            return;
        }
        if let Err(e) = url::Url::parse(url) {
            state.last_error = Some(format!("Invalid URL: {e}"));
            return;
        }
        let mut new_settings = server.settings();
        new_settings.relay = Some(RelaySettings {
            url: url.to_string(),
            insecure: state.insecure,
        });
        apply(state, server, toasts, new_settings);
    }
}

fn apply(
    state: &mut RelayWindowState,
    server: &Arc<ServerHandle>,
    toasts: &mut ToastList,
    new_settings: PersistentSettings,
) {
    match server.restart(new_settings.clone()) {
        Ok(_) => {
            if let Err(e) = new_settings.save() {
                toasts.error(format!("Relay applied; settings disk write failed: {e}"));
            } else if new_settings.relay.is_some() {
                toasts.success("Relay enabled");
            } else {
                toasts.info("Relay disabled");
            }
            state.close();
        }
        Err(e) => {
            let msg = format!("Could not apply relay: {e:#}");
            state.last_error = Some(msg.clone());
            toasts.error(msg);
        }
    }
}
