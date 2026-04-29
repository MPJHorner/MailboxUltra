//! HTML tab body. The egui chrome (device-size buttons + Source toggle +
//! "Open in browser") is drawn at the top; everything below is either the
//! native `WKWebView` (Rendered) or a syntect-highlighted source view
//! (Source).
//!
//! The body is intentionally NOT wrapped in `ScrollArea`: the WKWebView
//! handles its own scrolling natively, and an outer ScrollArea creates a
//! coordinate-system mismatch that drifts the WKWebView frame upward and
//! over the toolbar/tabs.

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
        // No HTML body — hide the WebKit view, fall back to text/plain.
        #[cfg(target_os = "macos")]
        if let Some(view) = ctx.native_html {
            view.set_visible(false);
        }
        egui::Frame::default()
            .inner_margin(egui::Margin::symmetric(24, 16))
            .show(ui, |ui| {
                if let Some(text) = &m.text {
                    ui.label(
                        RichText::new("This message has no HTML part. Showing text/plain.")
                            .small()
                            .color(ui.style().visuals.weak_text_color()),
                    );
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut text.clone())
                                    .desired_width(f32::INFINITY)
                                    .code_editor()
                                    .interactive(false),
                            );
                        });
                } else {
                    ui.label(
                        RichText::new("(no text or HTML body)")
                            .color(ui.style().visuals.weak_text_color()),
                    );
                }
            });
        return;
    };

    // Sub-control row: device-size buttons + Source toggle + Open in browser.
    egui::Frame::default()
        .inner_margin(egui::Margin {
            left: 24,
            right: 24,
            top: 12,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                device_chip(ui, state, DeviceSize::Desktop);
                device_chip(ui, state, DeviceSize::Tablet);
                device_chip(ui, state, DeviceSize::Mobile);
                ui.add_space(12.0);
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
                        .on_hover_text(
                            "Write the HTML to a temp file and shell to the system browser",
                        )
                        .clicked()
                    {
                        if let Ok(path) = write_to_temp(m.id, html) {
                            let _ = open::that(&path);
                        }
                    }
                });
            });
        });

    match state.sub_tab {
        HtmlSubTab::Rendered => {
            #[cfg(target_os = "macos")]
            if let Some(view) = ctx.native_html {
                // Body rect = whatever's left after the chrome above. We
                // explicitly allocate it so egui's layout sees it consumed
                // (otherwise auto-sizing would shrink the central panel).
                let pane = ui.available_rect_before_wrap();
                ui.allocate_rect(pane, egui::Sense::hover());
                let frame_rect = device_frame(pane, state.device);

                if state.device != DeviceSize::Desktop {
                    // Stage backdrop for the constrained-width modes.
                    ui.painter().rect_filled(
                        pane,
                        egui::CornerRadius::ZERO,
                        ui.style().visuals.faint_bg_color,
                    );
                    ui.painter().rect_stroke(
                        frame_rect,
                        egui::CornerRadius::same(10),
                        Stroke::new(1.0, ui.style().visuals.widgets.inactive.bg_stroke.color),
                        egui::StrokeKind::Inside,
                    );
                }
                view.set_frame(frame_rect);
                view.set_visible(true);
                view.load(m.id, html);
                return;
            }

            // Non-mac fallback — show source.
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(24, 8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Native HTML rendering is unavailable on this build. Showing source.",
                        )
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                    );
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            render_source(ui, html);
                        });
                });
        }
        HtmlSubTab::Source => {
            #[cfg(target_os = "macos")]
            if let Some(view) = ctx.native_html {
                view.set_visible(false);
            }
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(24, 8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            render_source(ui, html);
                        });
                });
        }
    }
}

fn device_chip(ui: &mut egui::Ui, state: &mut HtmlState, device: DeviceSize) {
    let active = state.device == device;
    let visuals = ui.style().visuals.clone();
    let accent = theme::accent(ui.ctx());

    let label_text = device.label();
    let dim_text = device.dim();
    let label_galley = ui.painter().layout_no_wrap(
        label_text.to_string(),
        egui::TextStyle::Button.resolve(ui.style()),
        if active {
            accent
        } else {
            visuals.text_color().gamma_multiply(0.85)
        },
    );
    let dim_galley = ui.painter().layout_no_wrap(
        dim_text.to_string(),
        egui::TextStyle::Small.resolve(ui.style()),
        visuals.weak_text_color(),
    );

    let pad_x = 12.0;
    let pad_y = 6.0;
    let inner_gap = 8.0;
    let dim_pad_x = 6.0;
    let dim_pad_y = 1.0;
    let label_size = label_galley.size();
    let dim_size = dim_galley.size();
    let dim_outer = egui::vec2(dim_size.x + dim_pad_x * 2.0, dim_size.y + dim_pad_y * 2.0);
    let inner_w = label_size.x + inner_gap + dim_outer.x;
    let inner_h = label_size.y.max(dim_outer.y);
    let outer = egui::vec2(inner_w + pad_x * 2.0, inner_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(outer, egui::Sense::click());

    let (fill, stroke) = if active {
        (accent.gamma_multiply(0.18), Stroke::new(1.0, accent))
    } else if response.hovered() {
        (
            visuals.widgets.hovered.bg_fill,
            Stroke::new(1.0, visuals.widgets.hovered.bg_stroke.color),
        )
    } else {
        (
            visuals.widgets.inactive.bg_fill,
            Stroke::new(1.0, visuals.widgets.inactive.bg_stroke.color),
        )
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        stroke,
        egui::StrokeKind::Inside,
    );

    let label_pos = egui::pos2(rect.left() + pad_x, rect.center().y - label_size.y / 2.0);
    ui.painter().galley(label_pos, label_galley, accent);

    let dim_left = label_pos.x + label_size.x + inner_gap;
    let dim_top = rect.center().y - dim_outer.y / 2.0;
    let dim_rect = egui::Rect::from_min_size(egui::pos2(dim_left, dim_top), dim_outer);
    ui.painter().rect_filled(
        dim_rect,
        egui::CornerRadius::same(255),
        if active {
            Color32::TRANSPARENT
        } else {
            visuals.faint_bg_color
        },
    );
    let dim_pos = egui::pos2(
        dim_rect.center().x - dim_size.x / 2.0,
        dim_rect.center().y - dim_size.y / 2.0,
    );
    ui.painter().galley(
        dim_pos,
        dim_galley,
        if active {
            accent
        } else {
            visuals.weak_text_color()
        },
    );

    if response.clicked() {
        state.device = device;
    }
}

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
