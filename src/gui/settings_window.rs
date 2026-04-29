//! Preferences window: every setting that used to be a CLI flag, plus
//! theme. Triggered by ⌘, or the gear button in the toolbar.
//!
//! "Apply" validates the form, persists to disk via PersistentSettings::save,
//! and calls ServerHandle::restart. "Cancel" discards the buffered edits.

use std::path::PathBuf;
use std::sync::Arc;

use egui::{RichText, Stroke};

use crate::server::ServerHandle;
use crate::settings::{Auth, PersistentSettings, RelaySettings, Theme};

use super::theme;
use super::toasts::ToastList;
use super::widgets;

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
        .default_size([580.0, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let Some(buffer) = state.buffer.as_mut() else {
                return;
            };
            section(ui, "Servers", |ui| {
                grid(ui, "servers-grid", |ui| {
                    label(ui, "SMTP port");
                    ui.add(field(&mut buffer.smtp_port).desired_width(120.0));
                    ui.end_row();
                    label(ui, "Bind address");
                    ui.add(field(&mut buffer.bind).desired_width(180.0));
                    ui.end_row();
                });
            });
            section(ui, "SMTP", |ui| {
                grid(ui, "smtp-grid", |ui| {
                    label(ui, "Hostname");
                    ui.add(field(&mut buffer.hostname).desired_width(280.0));
                    ui.end_row();
                    label(ui, "Max message size");
                    ui.horizontal(|ui| {
                        ui.add(field(&mut buffer.max_message_size_mib).desired_width(80.0));
                        ui.label(RichText::new("MiB").color(theme::muted_text_color(ui.ctx())));
                    });
                    ui.end_row();
                });
                ui.add_space(4.0);
                widgets::nice_checkbox(ui, &mut buffer.auth_required, "Require AUTH");
                if buffer.auth_required {
                    ui.add_space(2.0);
                    grid(ui, "smtp-auth-grid", |ui| {
                        label(ui, "User");
                        ui.add(field(&mut buffer.auth_user).desired_width(280.0));
                        ui.end_row();
                        label(ui, "Password");
                        ui.add(
                            egui::TextEdit::singleline(&mut buffer.auth_pass)
                                .password(true)
                                .desired_width(280.0),
                        );
                        ui.end_row();
                    });
                }
            });
            section(ui, "Capture", |ui| {
                grid(ui, "capture-grid", |ui| {
                    label(ui, "Buffer size");
                    ui.horizontal(|ui| {
                        ui.add(field(&mut buffer.buffer_size).desired_width(100.0));
                        ui.label(
                            RichText::new("messages").color(theme::muted_text_color(ui.ctx())),
                        );
                    });
                    ui.end_row();
                });
            });
            section(ui, "Relay", |ui| {
                widgets::nice_checkbox(
                    ui,
                    &mut buffer.relay_enabled,
                    "Forward each captured message upstream",
                );
                if buffer.relay_enabled {
                    ui.add_space(4.0);
                    grid(ui, "relay-grid", |ui| {
                        label(ui, "Upstream URL");
                        ui.add(
                            field(&mut buffer.relay_url)
                                .hint_text("smtp://relay.example.com:25")
                                .desired_width(360.0),
                        );
                        ui.end_row();
                    });
                    ui.add_space(2.0);
                    widgets::nice_checkbox(
                        ui,
                        &mut buffer.relay_insecure,
                        "Skip TLS certificate verification (dev only)",
                    );
                }
            });
            section(ui, "Logging", |ui| {
                widgets::nice_checkbox(
                    ui,
                    &mut buffer.log_file_enabled,
                    "Append every captured message as NDJSON to a log file",
                );
                if buffer.log_file_enabled {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            field(&mut buffer.log_file)
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
                    ui.label(RichText::new("Theme").color(theme::muted_text_color(ui.ctx())));
                    ui.add_space(8.0);
                    ui.radio_value(&mut buffer.theme, Theme::System, "System");
                    ui.radio_value(&mut buffer.theme, Theme::Dark, "Dark");
                    ui.radio_value(&mut buffer.theme, Theme::Light, "Light");
                });
            });

            if let Some(err) = &state.last_error {
                ui.add_space(6.0);
                ui.label(RichText::new(err).color(theme::DANGER).small());
            }
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Reset to defaults").clicked() {
                    reset_clicked = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let apply = egui::Button::new(
                        RichText::new("Apply").color(egui::Color32::WHITE).strong(),
                    )
                    .fill(theme::accent(ui.ctx()))
                    .stroke(Stroke::new(1.0, theme::accent(ui.ctx())));
                    if ui.add(apply).clicked() {
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
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title.to_uppercase())
                .small()
                .strong()
                .color(theme::dim_text_color(ui.ctx())),
        );
        // Thin underline that runs to the right edge of the available width,
        // anchoring the heading to the column rule below.
        let r = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [
                egui::pos2(r.left() + 6.0, r.center().y),
                egui::pos2(r.right(), r.center().y),
            ],
            Stroke::new(1.0, theme::border_color(ui.ctx())),
        );
    });
    ui.add_space(8.0);
    content(ui);
}

fn grid(ui: &mut egui::Ui, id: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([14.0, 8.0])
        .show(ui, content);
}

fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::muted_text_color(ui.ctx())));
}

/// `egui::TextEdit::singleline` is a builder, not a widget — wrap with this so
/// every field in the form gets the same surface treatment without needing to
/// hand-thread `desired_width` everywhere.
fn field(value: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(value)
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
