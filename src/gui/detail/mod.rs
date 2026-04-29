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
use egui::{Color32, RichText};

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
    pub window_height: f32,
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
    let timestamp = local.format("%Y-%m-%d %H:%M:%S").to_string();
    let size = humansize::format_size(m.size as u64, humansize::BINARY);

    ui.horizontal_wrapped(|ui| {
        pill(ui, "from", &from);
        ui.label("→");
        pill(ui, "to", &to_line);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(timestamp).small().monospace());
        ui.label("·");
        ui.label(RichText::new(size).small());
        if m.authenticated {
            ui.label("·");
            ui.label(
                RichText::new("AUTH")
                    .small()
                    .color(Color32::from_rgb(45, 212, 191)),
            );
        }
    });
    ui.add_space(8.0);
    let subject = m.subject.as_deref().unwrap_or("(no subject)");
    ui.heading(subject);
}

fn pill(ui: &mut egui::Ui, label: &str, value: &str) {
    let bg = ui.style().visuals.widgets.inactive.bg_fill;
    egui::Frame::group(ui.style())
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .stroke(egui::Stroke::NONE)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                );
                ui.label(RichText::new(value).strong());
            });
        });
}

fn draw_tabs(ui: &mut egui::Ui, m: &Message, selected: &mut DetailTab) {
    ui.horizontal(|ui| {
        for tab in DetailTab::ALL {
            let count = tab_meta(tab, m);
            let label = match count {
                Some(n) => format!("{} {}", tab.label(), n),
                None => tab.label().to_string(),
            };
            if ui.selectable_label(*selected == tab, label).clicked() {
                *selected = tab;
            }
        }
    });
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
