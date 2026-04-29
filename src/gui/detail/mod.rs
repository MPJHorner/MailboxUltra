//! Right pane: detail view for the selected message.
//!
//! The HTML tab will be replaced in Phase 6 by a native WKWebView embedded
//! in the eframe window; right now it falls back to the source view so the
//! pane is fully usable.

pub mod attachments;
pub mod headers;
pub mod html;
pub mod release;
pub mod source;
pub mod text;

use std::sync::Arc;

use chrono::{DateTime, Local};
use egui::{Color32, RichText, Sense, Stroke};

use crate::gui::theme;
use crate::message::Message;
use crate::server::ServerHandle;

use super::toasts::ToastList;

#[derive(Default)]
pub struct DetailState {
    pub selected_tab: DetailTab,
    pub release: release::ReleaseState,
    pub html: html::HtmlState,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetailTab {
    #[default]
    Html,
    Text,
    Headers,
    Attachments,
    Source,
    Release,
}

impl DetailTab {
    pub fn label(self) -> &'static str {
        match self {
            DetailTab::Html => "HTML",
            DetailTab::Text => "Text",
            DetailTab::Headers => "Headers",
            DetailTab::Attachments => "Attachments",
            DetailTab::Source => "Source",
            DetailTab::Release => "Release",
        }
    }
    pub const ALL: [DetailTab; 6] = [
        DetailTab::Html,
        DetailTab::Text,
        DetailTab::Headers,
        DetailTab::Attachments,
        DetailTab::Source,
        DetailTab::Release,
    ];
}

pub struct DetailContext<'a> {
    pub server: &'a Arc<ServerHandle>,
    pub toasts: &'a mut ToastList,
    #[cfg(target_os = "macos")]
    pub native_html: Option<&'a super::native_html::NativeHtmlView>,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut DetailState,
    message: Option<&Message>,
    ctx: &mut DetailContext<'_>,
) {
    let Some(m) = message else {
        empty(ui);
        return;
    };
    draw_header(ui, m);
    ui.separator();
    draw_tabs(ui, m, &mut state.selected_tab);
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt(("detail-body", state.selected_tab))
        .auto_shrink([false; 2])
        .show(ui, |ui| match state.selected_tab {
            DetailTab::Html => html::render(ui, &mut state.html, m, ctx),
            DetailTab::Text => text::render(ui, m),
            DetailTab::Headers => headers::render(ui, m),
            DetailTab::Attachments => attachments::render(ui, m, ctx),
            DetailTab::Source => source::render(ui, m),
            DetailTab::Release => release::render(ui, &mut state.release, m, ctx),
        });

    // Hide the WKWebView whenever the HTML tab isn't the active tab. The
    // tab body above only ever asks the view to be visible when on Html.
    #[cfg(target_os = "macos")]
    if state.selected_tab != DetailTab::Html {
        if let Some(view) = ctx.native_html {
            view.set_visible(false);
        }
    }
}

fn empty(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(72.0);
        ui.label(
            RichText::new("Select a message to inspect")
                .color(Color32::from_rgb(150, 150, 150))
                .size(15.0),
        );
    });
}

fn draw_header(ui: &mut egui::Ui, m: &Message) {
    let from = m
        .from
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_else(|| m.envelope_from.clone());
    let to_line = if !m.to.is_empty() {
        m.to.iter()
            .map(|a| a.address.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        m.envelope_to.join(", ")
    };
    let local: DateTime<Local> = m.received_at.with_timezone(&Local);
    let timestamp = local.format("%a %b %-d %Y · %H:%M:%S").to_string();
    let size = humansize::format_size(m.size as u64, humansize::BINARY);
    let accent = theme::accent(ui.ctx());

    ui.horizontal_wrapped(|ui| {
        from_pill(ui, &from, accent);
        ui.label(RichText::new("→").color(ui.style().visuals.weak_text_color()));
        plain_pill(ui, &to_line);
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(timestamp)
                .small()
                .monospace()
                .color(ui.style().visuals.weak_text_color()),
        );
        ui.label(RichText::new("·").color(ui.style().visuals.weak_text_color()));
        ui.label(
            RichText::new(size)
                .small()
                .monospace()
                .color(ui.style().visuals.weak_text_color()),
        );
        if m.authenticated {
            ui.add_space(4.0);
            auth_pill(ui, accent);
        }
    });
    ui.add_space(10.0);
    let subject = m.subject.as_deref().unwrap_or("(no subject)");
    ui.add(
        egui::Label::new(
            RichText::new(subject)
                .size(22.0)
                .color(ui.style().visuals.text_color()),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

fn from_pill(ui: &mut egui::Ui, value: &str, accent: Color32) {
    let bg = ui.style().visuals.faint_bg_color;
    egui::Frame::group(ui.style())
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(12, 4))
        .stroke(Stroke::new(
            1.0,
            ui.style().visuals.widgets.noninteractive.bg_stroke.color,
        ))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(value).color(accent).monospace().size(13.0))
                    .selectable(true),
            );
        });
}

fn plain_pill(ui: &mut egui::Ui, value: &str) {
    let bg = ui.style().visuals.faint_bg_color;
    egui::Frame::group(ui.style())
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(12, 4))
        .stroke(Stroke::new(
            1.0,
            ui.style().visuals.widgets.noninteractive.bg_stroke.color,
        ))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .color(ui.style().visuals.text_color())
                        .monospace()
                        .size(13.0),
                )
                .selectable(true),
            );
        });
}

fn auth_pill(ui: &mut egui::Ui, accent: Color32) {
    egui::Frame::group(ui.style())
        .fill(accent.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, accent))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new("AUTH")
                    .color(accent)
                    .strong()
                    .small()
                    .text_style(egui::TextStyle::Small),
            );
        });
}

fn draw_tabs(ui: &mut egui::Ui, m: &Message, selected: &mut DetailTab) {
    let accent = theme::accent(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for tab in DetailTab::ALL {
            let count = tab_meta(tab, m);
            let active = *selected == tab;
            if tab_button(ui, tab.label(), count, active, accent).clicked() {
                *selected = tab;
            }
        }
    });
}

/// One tab button: text + optional small badge + accent underline when
/// active. Hand-drawn so we can control the underline + badge styling.
fn tab_button(
    ui: &mut egui::Ui,
    label: &str,
    count: Option<usize>,
    active: bool,
    accent: Color32,
) -> egui::Response {
    let visuals = ui.style().visuals.clone();
    let label_color = if active {
        accent
    } else {
        visuals.text_color().gamma_multiply(0.85)
    };
    let badge_text = count.map(|n| n.to_string());

    // Measure: label width + badge width + paddings.
    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::TextStyle::Button.resolve(ui.style()),
        label_color,
    );
    let label_size = label_galley.size();

    let pad_x = 12.0;
    let pad_y = 8.0;
    let underline_h = 2.0;

    let badge_size = badge_text.as_ref().map(|t| {
        let galley = ui.painter().layout_no_wrap(
            t.clone(),
            egui::TextStyle::Small.resolve(ui.style()),
            visuals.weak_text_color(),
        );
        galley.size()
    });

    let badge_pad_x = 8.0;
    let badge_pad_y = 2.0;
    let badge_outer = badge_size.map(|s| {
        egui::vec2(
            s.x + badge_pad_x * 2.0,
            (s.y + badge_pad_y * 2.0).max(label_size.y),
        )
    });
    let inner_gap = if badge_outer.is_some() { 6.0 } else { 0.0 };

    let total_w = label_size.x + inner_gap + badge_outer.map(|s| s.x).unwrap_or(0.0);
    let total_h = label_size.y;
    let outer_size = egui::vec2(total_w + pad_x * 2.0, total_h + pad_y * 2.0 + underline_h);
    let (rect, response) = ui.allocate_exact_size(outer_size, Sense::click());

    if response.hovered() && !active {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(6),
            visuals.widgets.hovered.bg_fill.gamma_multiply(0.6),
        );
    }

    let label_pos = egui::pos2(rect.left() + pad_x, rect.top() + pad_y);
    ui.painter().galley(label_pos, label_galley, label_color);

    if let (Some(text), Some(b_outer)) = (&badge_text, badge_outer) {
        let badge_left = label_pos.x + label_size.x + inner_gap;
        let badge_top = rect.top() + pad_y + (label_size.y - b_outer.y) / 2.0;
        let badge_rect = egui::Rect::from_min_size(egui::pos2(badge_left, badge_top), b_outer);
        ui.painter().rect_filled(
            badge_rect,
            egui::CornerRadius::same(255),
            visuals.faint_bg_color,
        );
        let badge_text_galley = ui.painter().layout_no_wrap(
            text.clone(),
            egui::TextStyle::Small.resolve(ui.style()),
            visuals.weak_text_color(),
        );
        let bt_size = badge_text_galley.size();
        let bt_pos = egui::pos2(
            badge_rect.center().x - bt_size.x / 2.0,
            badge_rect.center().y - bt_size.y / 2.0,
        );
        ui.painter()
            .galley(bt_pos, badge_text_galley, visuals.weak_text_color());
    }

    if active {
        let y = rect.bottom() - underline_h;
        let underline_rect =
            egui::Rect::from_min_max(egui::pos2(rect.left() + 2.0, y), rect.right_bottom());
        ui.painter()
            .rect_filled(underline_rect, egui::CornerRadius::same(2), accent);
    }

    response
}

fn tab_meta(tab: DetailTab, m: &Message) -> Option<usize> {
    match tab {
        DetailTab::Headers => Some(m.headers.len()),
        DetailTab::Attachments => {
            if m.attachments.is_empty() {
                None
            } else {
                Some(m.attachments.len())
            }
        }
        _ => None,
    }
}
