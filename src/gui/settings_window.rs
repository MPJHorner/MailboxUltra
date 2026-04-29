//! Preferences window: every setting that used to be a CLI flag, plus
//! theme. Triggered by ⌘, or the gear button in the toolbar.
//!
//! "Apply" validates the form, persists to disk via PersistentSettings::save,
//! and calls ServerHandle::restart. "Cancel" discards the buffered edits.

use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, RichText};

use crate::server::ServerHandle;
use crate::settings::{Auth, PersistentSettings, RelaySettings, Theme};

use super::toasts::ToastList;

#[derive(Clone)]
pub struct SettingsBuffer {
    pub smtp_port: String,
    pub bind: String,
    pub hostname: String,
    pub max_message_size_mib: String,
    pub buffer_size: String,
    pub auth_required: bool,
    pub auth_user: String,
    pub auth_pass: String,
    pub relay_enabled: bool,
    pub relay_url: String,
    pub relay_insecure: bool,
    pub log_file_enabled: bool,
    pub log_file: String,
    pub theme: Theme,
}

impl SettingsBuffer {
    pub fn from_persistent(s: &PersistentSettings) -> Self {
        Self {
            smtp_port: s.smtp_port.to_string(),
            bind: s.bind.to_string(),
            hostname: s.hostname.clone(),
            max_message_size_mib: format!("{:.1}", s.max_message_size as f64 / (1024.0 * 1024.0)),
            buffer_size: s.buffer_size.to_string(),
            auth_required: s.auth.is_some(),
            auth_user: s.auth.as_ref().map(|a| a.user.clone()).unwrap_or_default(),
            auth_pass: s.auth.as_ref().map(|a| a.pass.clone()).unwrap_or_default(),
            relay_enabled: s.relay.is_some(),
            relay_url: s.relay.as_ref().map(|r| r.url.clone()).unwrap_or_default(),
            relay_insecure: s.relay.as_ref().map(|r| r.insecure).unwrap_or(false),
            log_file_enabled: s.log_file.is_some(),
            log_file: s
                .log_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            theme: s.theme,
        }
    }

    pub fn to_persistent(&self, base: &PersistentSettings) -> Result<PersistentSettings, String> {
        let smtp_port: u16 = self
            .smtp_port
            .trim()
            .parse()
            .map_err(|_| format!("Invalid SMTP port: '{}'", self.smtp_port))?;
        let bind = self
            .bind
            .trim()
            .parse()
            .map_err(|_| format!("Invalid bind address: '{}'", self.bind))?;
        if self.hostname.trim().is_empty() {
            return Err("Hostname must not be empty".into());
        }
        let max_mib: f64 = self
            .max_message_size_mib
            .trim()
            .parse()
            .map_err(|_| format!("Invalid max size: '{}'", self.max_message_size_mib))?;
        if max_mib <= 0.0 {
            return Err("Max message size must be greater than 0".into());
        }
        let max_message_size = (max_mib * 1024.0 * 1024.0) as usize;
        let buffer_size: usize = self
            .buffer_size
            .trim()
            .parse()
            .map_err(|_| format!("Invalid buffer size: '{}'", self.buffer_size))?;
        if buffer_size == 0 {
            return Err("Buffer size must be greater than 0".into());
        }
        let auth = if self.auth_required {
            if self.auth_user.trim().is_empty() {
                return Err("Auth user must not be empty when AUTH is enabled".into());
            }
            Some(Auth {
                user: self.auth_user.clone(),
                pass: self.auth_pass.clone(),
            })
        } else {
            None
        };
        let relay = if self.relay_enabled {
            let url = self.relay_url.trim();
            if url.is_empty() {
                return Err("Relay URL must not be empty when relay is enabled".into());
            }
            // Light validation; ServerHandle::restart does the real parse.
            url::Url::parse(url).map_err(|e| format!("Invalid relay URL: {e}"))?;
            Some(RelaySettings {
                url: url.to_string(),
                insecure: self.relay_insecure,
            })
        } else {
            None
        };
        let log_file = if self.log_file_enabled && !self.log_file.trim().is_empty() {
            Some(PathBuf::from(self.log_file.trim()))
        } else {
            None
        };
        Ok(PersistentSettings {
            schema_version: base.schema_version,
            smtp_port,
            bind,
            hostname: self.hostname.clone(),
            max_message_size,
            auth,
            buffer_size,
            relay,
            log_file,
            theme: self.theme,
        })
    }
}

#[derive(Default)]
pub struct SettingsWindowState {
    pub open: bool,
    pub buffer: Option<SettingsBuffer>,
    pub last_error: Option<String>,
}

impl SettingsWindowState {
    pub fn open_with(&mut self, current: &PersistentSettings) {
        self.open = true;
        self.buffer = Some(SettingsBuffer::from_persistent(current));
        self.last_error = None;
    }
    pub fn close(&mut self) {
        self.open = false;
        self.buffer = None;
        self.last_error = None;
    }
}

pub fn render(
    ctx: &egui::Context,
    state: &mut SettingsWindowState,
    server: &Arc<ServerHandle>,
    toasts: &mut ToastList,
) {
    if !state.open {
        return;
    }
    let mut keep_open = true;
    let mut apply_clicked = false;
    let mut cancel_clicked = false;
    let mut reset_clicked = false;

    egui::Window::new(RichText::new("Preferences").strong())
        .open(&mut keep_open)
        .resizable(false)
        .collapsible(false)
        .default_size([560.0, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let Some(buffer) = state.buffer.as_mut() else {
                return;
            };
            section(ui, "Servers", |ui| {
                grid(ui, "servers-grid", |ui| {
                    label(ui, "SMTP port");
                    ui.text_edit_singleline(&mut buffer.smtp_port);
                    ui.end_row();
                    label(ui, "Bind address");
                    ui.text_edit_singleline(&mut buffer.bind);
                    ui.end_row();
                });
            });
            section(ui, "SMTP", |ui| {
                grid(ui, "smtp-grid", |ui| {
                    label(ui, "Hostname");
                    ui.text_edit_singleline(&mut buffer.hostname);
                    ui.end_row();
                    label(ui, "Max msg size (MiB)");
                    ui.text_edit_singleline(&mut buffer.max_message_size_mib);
                    ui.end_row();
                    label(ui, "Require AUTH");
                    ui.checkbox(&mut buffer.auth_required, "");
                    ui.end_row();
                    if buffer.auth_required {
                        label(ui, "User");
                        ui.text_edit_singleline(&mut buffer.auth_user);
                        ui.end_row();
                        label(ui, "Password");
                        ui.add(egui::TextEdit::singleline(&mut buffer.auth_pass).password(true));
                        ui.end_row();
                    }
                });
            });
            section(ui, "Capture", |ui| {
                grid(ui, "capture-grid", |ui| {
                    label(ui, "Buffer size (messages)");
                    ui.text_edit_singleline(&mut buffer.buffer_size);
                    ui.end_row();
                });
            });
            section(ui, "Relay", |ui| {
                ui.checkbox(
                    &mut buffer.relay_enabled,
                    "Forward each captured message to upstream",
                );
                if buffer.relay_enabled {
                    grid(ui, "relay-grid", |ui| {
                        label(ui, "URL");
                        ui.add(
                            egui::TextEdit::singleline(&mut buffer.relay_url)
                                .hint_text("smtp://relay.example.com:25"),
                        );
                        ui.end_row();
                    });
                    ui.checkbox(
                        &mut buffer.relay_insecure,
                        "Skip TLS certificate verification (dev only)",
                    );
                }
            });
            section(ui, "Logging", |ui| {
                ui.checkbox(
                    &mut buffer.log_file_enabled,
                    "Append every captured message as NDJSON to a log file",
                );
                if buffer.log_file_enabled {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut buffer.log_file)
                                .hint_text("/path/to/messages.ndjson")
                                .desired_width(360.0),
                        );
                        if ui.button("Browse…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("messages.ndjson")
                                .save_file()
                            {
                                buffer.log_file = path.display().to_string();
                            }
                        }
                    });
                }
            });
            section(ui, "Appearance", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    ui.radio_value(&mut buffer.theme, Theme::System, "System");
                    ui.radio_value(&mut buffer.theme, Theme::Dark, "Dark");
                    ui.radio_value(&mut buffer.theme, Theme::Light, "Light");
                });
            });

            if let Some(err) = &state.last_error {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(err)
                        .color(Color32::from_rgb(248, 113, 113))
                        .small(),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Reset to defaults").clicked() {
                    reset_clicked = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Apply").clicked() {
                        apply_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            });
        });

    if !keep_open || cancel_clicked {
        state.close();
        return;
    }
    if reset_clicked {
        state.buffer = Some(SettingsBuffer::from_persistent(
            &PersistentSettings::default(),
        ));
        state.last_error = None;
    }
    if apply_clicked {
        let current = server.settings();
        let buffer = state.buffer.clone();
        if let Some(buffer) = buffer {
            match buffer.to_persistent(&current) {
                Ok(new_settings) => match server.restart(new_settings.clone()) {
                    Ok(report) => {
                        if let Err(e) = new_settings.save() {
                            toasts.error(format!(
                                "Settings saved in memory but disk write failed: {e}"
                            ));
                        } else {
                            describe_report(toasts, &report);
                        }
                        state.close();
                    }
                    Err(e) => {
                        let msg = format!("Could not apply: {e:#}");
                        state.last_error = Some(msg.clone());
                        toasts.error(msg);
                    }
                },
                Err(e) => {
                    state.last_error = Some(e.clone());
                    toasts.error(e);
                }
            }
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(title)
            .strong()
            .color(ui.style().visuals.weak_text_color()),
    );
    ui.separator();
    content(ui);
}

fn grid(ui: &mut egui::Ui, id: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, content);
}

fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(ui.style().visuals.weak_text_color()));
}

fn describe_report(toasts: &mut ToastList, report: &crate::server::RestartReport) {
    if report.nothing_changed() {
        toasts.info("Settings saved (no servers needed restarting)");
        return;
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(addr) = report.smtp_restarted {
        parts.push(format!("SMTP rebound to :{}", addr.port()));
        if report.messages_preserved > 0 {
            parts.push(format!("{} messages preserved", report.messages_preserved));
        }
    }
    if report.log_changed {
        parts.push("log writer updated".into());
    }
    if report.relay_changed {
        parts.push("relay updated".into());
    }
    toasts.success(parts.join(" · "));
}
