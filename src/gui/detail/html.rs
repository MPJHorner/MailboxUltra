//! HTML tab.
//!
//! On macOS the rendered preview lives inside a real `WKWebView` embedded as
//! a child `NSView` of the eframe window (see `gui::native_html`). egui
//! draws the device-size buttons + "Source" toggle and reserves a rect; the
//! WKWebView is repositioned to overlap that rect each frame.
//!
//! When the message has no HTML body, the tab falls back to plain text.

use std::path::PathBuf;

use egui::{Color32, RichText, Stroke};
use egui_extras::syntax_highlighting::{self, CodeTheme};

use crate::gui::theme;
use crate::message::Message;

use super::DetailContext;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HtmlSubTab {
    #[default]
    Rendered,
    Source,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceSize {
    #[default]
    Desktop,
    Tablet,
    Mobile,
}

impl DeviceSize {
    /// Width in egui points to constrain the WKWebView to. `None` means use
    /// the full pane width.
    pub fn width(self) -> Option<f32> {
        match self {
            DeviceSize::Desktop => None,
            DeviceSize::Tablet => Some(820.0),
            DeviceSize::Mobile => Some(390.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DeviceSize::Desktop => "Desktop",
            DeviceSize::Tablet => "iPad",
            DeviceSize::Mobile => "Mobile",
        }
    }

    pub fn dim(self) -> &'static str {
        match self {
            DeviceSize::Desktop => "full",
            DeviceSize::Tablet => "820",
            DeviceSize::Mobile => "390",
        }
    }
}

#[derive(Default)]
pub struct HtmlState {
    pub sub_tab: HtmlSubTab,
    pub device: DeviceSize,
}

pub fn render(ui: &mut egui::Ui, state: &mut HtmlState, m: &Message, ctx: &mut DetailContext<'_>) {
    let Some(html) = m.html.as_ref() else {
        #[cfg(target_os = "macos")]
        if let Some(view) = ctx.native_html {
            view.set_visible(false);
        }
        if let Some(text) = &m.text {
            ui.label(
                RichText::new("This message has no HTML part. Showing text/plain.")
                    .small()
                    .color(ui.style().visuals.weak_text_color()),
            );
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::multiline(&mut text.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(28)
                    .code_editor()
                    .interactive(false),
            );
        } else {
            ui.label(
                RichText::new("(no text or HTML body)").color(ui.style().visuals.weak_text_color()),
            );
        }
        return;
    };

    // Top control row: device-size buttons (left) + Source toggle + Open in
    // browser (right).
    ui.horizontal(|ui| {
        device_button(ui, state, DeviceSize::Desktop);
        device_button(ui, state, DeviceSize::Tablet);
        device_button(ui, state, DeviceSize::Mobile);
        ui.add_space(8.0);
        let in_source = state.sub_tab == HtmlSubTab::Source;
        if ui.selectable_label(in_source, "Source").clicked() {
            state.sub_tab = if in_source {
                HtmlSubTab::Rendered
            } else {
                HtmlSubTab::Source
            };
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Open in browser")
                .on_hover_text("Write the HTML to a temp file and shell to the system browser")
                .clicked()
            {
                match write_to_temp(m.id, html) {
                    Ok(path) => {
                        let _ = open::that(&path);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to write HTML preview to temp");
                    }
                }
            }
        });
    });
    ui.add_space(8.0);

    match state.sub_tab {
        HtmlSubTab::Rendered => {
            #[cfg(target_os = "macos")]
            {
                if let Some(view) = ctx.native_html {
                    let pane = ui.available_rect_before_wrap();
                    let frame = device_frame(pane, state.device);
                    // Allocate the space so egui's layout knows about it.
                    ui.allocate_rect(pane, egui::Sense::hover());
                    // Subtle stage background around constrained-width modes.
                    if state.device != DeviceSize::Desktop {
                        ui.painter().rect_filled(
                            pane,
                            egui::CornerRadius::same(8),
                            ui.style().visuals.faint_bg_color,
                        );
                        // Border around the active web view for definition.
                        ui.painter().rect_stroke(
                            frame,
                            egui::CornerRadius::same(10),
                            Stroke::new(1.0, ui.style().visuals.widgets.inactive.bg_fill),
                            egui::StrokeKind::Inside,
                        );
                    }
                    view.set_frame(frame);
                    view.set_visible(true);
                    view.load(m.id, html);
                    return;
                }
            }
            ui.label(
                RichText::new(
                    "Native HTML rendering is unavailable on this build. Showing source.",
                )
                .small()
                .color(ui.style().visuals.weak_text_color()),
            );
            render_source(ui, html);
        }
        HtmlSubTab::Source => {
            #[cfg(target_os = "macos")]
            if let Some(view) = ctx.native_html {
                view.set_visible(false);
            }
            render_source(ui, html);
        }
    }
}

fn device_button(ui: &mut egui::Ui, state: &mut HtmlState, device: DeviceSize) {
    let active = state.device == device;
    let accent = theme::accent(ui.ctx());
    let response = ui.scope(|ui| {
        let visuals = ui.style().visuals.clone();
        if active {
            ui.style_mut().visuals.widgets.inactive.bg_fill = visuals.faint_bg_color;
            ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, accent);
            ui.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, accent);
        }
        ui.button(format!("{} {}", device.label(), device.dim()))
    });
    if response.inner.clicked() {
        state.device = device;
    }
}

/// Compute the egui rect that the WKWebView should occupy for the given
/// device size. For Tablet/Mobile we centre the constrained-width view
/// horizontally inside the pane, with breathing room above and below so the
/// "phone" floats on the page.
fn device_frame(pane: egui::Rect, device: DeviceSize) -> egui::Rect {
    match device.width() {
        None => pane,
        Some(target_width) => {
            let pad_y = 16.0;
            let usable_width = pane.width().min(target_width);
            let x_center = pane.center().x;
            let left = x_center - usable_width / 2.0;
            let right = x_center + usable_width / 2.0;
            egui::Rect::from_min_max(
                egui::pos2(left, pane.top() + pad_y),
                egui::pos2(right, pane.bottom() - pad_y),
            )
        }
    }
}

fn render_source(ui: &mut egui::Ui, html: &str) {
    let theme = CodeTheme::from_style(ui.style());
    syntax_highlighting::code_view_ui(ui, &theme, html, "html");
}

fn write_to_temp(id: uuid::Uuid, html: &str) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join("MailBoxUltra-preview");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.html"));
    std::fs::write(&path, html.as_bytes())?;
    Ok(path)
}

#[allow(dead_code)]
const _: Color32 = Color32::TRANSPARENT;
