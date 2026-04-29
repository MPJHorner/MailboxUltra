//! Right pane: detail view for the selected message.
//!
//! Layout:
//!
//! ```text
//! ┌──────────── detail-chrome (fixed height) ────────────┐
//! │ from-pill → to-pill   timestamp · size · AUTH       │
//! │ Subject heading                                     │
//! │ ─────────────                                        │
//! │ HTML  Text  Headers 9  Attachments 2  Source  …    │
//! └──────────────────────────────────────────────────────┘
//! ┌──────────── body (the rest of the central panel) ───┐
//! │                                                     │
//! │   tab-specific content (WKWebView for HTML,         │
//! │   ScrollArea for Text / Source / Headers / etc.)    │
//! │                                                     │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! The chrome region is a `Panel::top` inside the central panel so it never
//! scrolls; each tab body manages its own scrolling. The HTML tab does NOT
//! wrap itself in a ScrollArea — the WKWebView scrolls itself, and wrapping
//! it in egui's ScrollArea introduces a coordinate-system mismatch that
//! drifts the WKWebView frame upward and over the chrome.

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
    tracing::trace!(
        "detail::render entry: max_rect={:?} selected={}",
        ui.max_rect(),
        message.is_some(),
    );
    let Some(m) = message else {
        empty(ui);
        return;
    };

    // Chrome (header + tabs) as a top panel inside the central panel.
    // egui's `Panel::top.show_inside` is the only pattern in this codebase
    // that reliably advances the parent cursor — see the
    // `advance_cursor_after_rect` call in egui's `show_inside_dyn`.
    let chrome_frame = egui::Frame::default()
        .fill(ui.style().visuals.window_fill)
        .stroke(egui::Stroke::NONE)
        .inner_margin(egui::Margin {
            left: 24,
            right: 24,
            top: 18,
            bottom: 12,
        });
    let chrome_response = egui::Panel::top("detail-chrome")
        .resizable(false)
        .show_separator_line(false)
        .frame(chrome_frame)
        .show_inside(ui, |ui| {
            draw_header(ui, m);
            ui.add_space(12.0);
            draw_tabs(ui, m, &mut state.selected_tab);
        });

    let chrome_rect = chrome_response.response.rect;
    tracing::trace!(
        "detail layout: full={:?} chrome={:?} cursor={:?}",
        ui.max_rect(),
        chrome_rect,
        ui.cursor(),
    );

    // Hairline below the chrome.
    let bottom_y = chrome_rect.bottom() - 0.5;
    ui.painter().line_segment(
        [
            egui::pos2(chrome_rect.left(), bottom_y),
            egui::pos2(chrome_rect.right(), bottom_y),
        ],
        egui::Stroke::new(
            1.0,
            ui.style().visuals.widgets.noninteractive.bg_stroke.color,
        ),
    );

    // Body — the rest of the central panel after the chrome panel was
    // drawn. egui has already advanced our cursor past the chrome.
    match state.selected_tab {
        DetailTab::Html => html::render(ui, &mut state.html, m, ctx),
        DetailTab::Text => with_padded_scroll(ui, "text", |ui| text::render(ui, m)),
        DetailTab::Headers => with_padded_scroll(ui, "headers", |ui| headers::render(ui, m)),
        DetailTab::Attachments => {
            with_padded_scroll(ui, "attachments", |ui| attachments::render(ui, m, ctx))
        }
        DetailTab::Source => with_padded_scroll(ui, "source", |ui| source::render(ui, m)),
        DetailTab::Release => with_padded_scroll(ui, "release", |ui| {
            release::render(ui, &mut state.release, m, ctx)
        }),
    }

    // Hide the WKWebView whenever the active tab isn't HTML so it doesn't
    // float over a different tab's content.
    #[cfg(target_os = "macos")]
    if state.selected_tab != DetailTab::Html {
        if let Some(view) = ctx.native_html {
            view.set_visible(false);
        }
    }
}

fn with_padded_scroll(ui: &mut egui::Ui, salt: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .id_salt(salt)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(24, 16))
                .show(ui, content);
        });
}

fn empty(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(96.0);
        ui.label(
            RichText::new("Select a message to inspect")
                .size(15.0)
                .color(ui.style().visuals.weak_text_color()),
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
        ui.add_space(8.0);
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
    ui.add_space(8.0);
    let subject = m.subject.as_deref().unwrap_or("(no subject)");
    ui.add(
        egui::Label::new(
            RichText::new(subject)
                .size(22.0)
                .strong()
                .color(ui.style().visuals.text_color()),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

fn from_pill(ui: &mut egui::Ui, value: &str, accent: Color32) {
    egui::Frame::default()
        .fill(ui.style().visuals.faint_bg_color)
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
    egui::Frame::default()
        .fill(ui.style().visuals.faint_bg_color)
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
    egui::Frame::default()
        .fill(accent.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, accent))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new("AUTH").color(accent).strong().small());
        });
}

fn draw_tabs(ui: &mut egui::Ui, m: &Message, selected: &mut DetailTab) {
    let accent = theme::accent(ui.ctx());
    let separator_color = ui.style().visuals.widgets.noninteractive.bg_stroke.color;

    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for tab in DetailTab::ALL {
                let count = tab_meta(tab, m);
                let active = *selected == tab;
                if tab_button(ui, tab.label(), count, active, accent).clicked() {
                    *selected = tab;
                }
            }
        })
        .response;

    // Bottom hairline under the entire tab strip — gives the active-tab
    // accent underline something to sit on.
    ui.painter().line_segment(
        [
            egui::pos2(response.rect.left(), response.rect.bottom()),
            egui::pos2(response.rect.right() + 200.0, response.rect.bottom()),
        ],
        Stroke::new(1.0, separator_color),
    );
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

    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::TextStyle::Button.resolve(ui.style()),
        label_color,
    );
    let label_size = label_galley.size();

    let pad_x = 14.0;
    let pad_y = 10.0;
    let underline_h = 2.0;

    let badge_size = badge_text.as_ref().map(|t| {
        ui.painter()
            .layout_no_wrap(
                t.clone(),
                egui::TextStyle::Small.resolve(ui.style()),
                visuals.weak_text_color(),
            )
            .size()
    });
    let badge_pad_x = 6.0;
    let badge_pad_y = 1.0;
    let badge_outer = badge_size
        .map(|s| egui::vec2((s.x + badge_pad_x * 2.0).max(18.0), s.y + badge_pad_y * 2.0));
    let inner_gap = if badge_outer.is_some() { 6.0 } else { 0.0 };

    let total_w = label_size.x + inner_gap + badge_outer.map(|s| s.x).unwrap_or(0.0);
    let total_h = label_size.y;
    let outer_size = egui::vec2(total_w + pad_x * 2.0, total_h + pad_y * 2.0 + underline_h);
    let (rect, response) = ui.allocate_exact_size(outer_size, Sense::click());

    if response.hovered() && !active {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(6),
            visuals.widgets.hovered.bg_fill.gamma_multiply(0.5),
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
        let underline_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 4.0, y),
            egui::pos2(rect.right() - 4.0, rect.bottom()),
        );
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
