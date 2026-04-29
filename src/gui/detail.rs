//! Right pane: detail view for the selected message. Phase 4 wires the
//! header strip and a placeholder body; Phase 5 fills in the real tabs.

use chrono::{DateTime, Local};
use egui::{Color32, RichText};

use crate::message::Message;

#[derive(Default)]
pub struct DetailState {
    pub selected_tab: DetailTab,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
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

pub fn render(ui: &mut egui::Ui, state: &mut DetailState, message: Option<&Message>) {
    let Some(m) = message else {
        empty(ui);
        return;
    };
    draw_header(ui, m);
    ui.separator();
    draw_tabs(ui, &mut state.selected_tab);
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            placeholder_body(ui, state.selected_tab, m);
        });
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
            ui.label(RichText::new("AUTH").small().color(Color32::from_rgb(45, 212, 191)));
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

fn draw_tabs(ui: &mut egui::Ui, selected: &mut DetailTab) {
    ui.horizontal(|ui| {
        for tab in DetailTab::ALL {
            let is_selected = *selected == tab;
            if ui.selectable_label(is_selected, tab.label()).clicked() {
                *selected = tab;
            }
        }
    });
}

fn placeholder_body(ui: &mut egui::Ui, tab: DetailTab, m: &Message) {
    match tab {
        DetailTab::Html => {
            if m.html.is_some() {
                ui.label(
                    RichText::new("HTML preview is wired in a later step.")
                        .color(ui.style().visuals.weak_text_color()),
                );
            } else if let Some(text) = &m.text {
                ui.add(
                    egui::TextEdit::multiline(&mut text.clone())
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .code_editor()
                        .interactive(false),
                );
            } else {
                ui.label("(no text or HTML body)");
            }
        }
        DetailTab::Text => {
            let body = m
                .text
                .clone()
                .unwrap_or_else(|| "(no text/plain part)".into());
            ui.add(
                egui::TextEdit::multiline(&mut body.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .code_editor()
                    .interactive(false),
            );
        }
        DetailTab::Headers => {
            for (k, v) in &m.headers {
                ui.horizontal_top(|ui| {
                    ui.add_sized(
                        [180.0, 18.0],
                        egui::Label::new(RichText::new(k).strong().monospace()).selectable(true),
                    );
                    ui.add(egui::Label::new(RichText::new(v).monospace()).selectable(true));
                });
                ui.add_space(2.0);
            }
        }
        DetailTab::Attachments => {
            if m.attachments.is_empty() {
                ui.label("(no attachments)");
            } else {
                for att in &m.attachments {
                    ui.horizontal(|ui| {
                        let name = att.filename.clone().unwrap_or_else(|| "(unnamed)".into());
                        ui.label(RichText::new(name).strong());
                        ui.label(RichText::new(&att.content_type).weak());
                        ui.label(RichText::new(humansize::format_size(att.size as u64, humansize::BINARY)).weak());
                    });
                }
            }
        }
        DetailTab::Source => {
            let s = String::from_utf8_lossy(&m.raw).into_owned();
            ui.add(
                egui::TextEdit::multiline(&mut s.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(40)
                    .code_editor()
                    .interactive(false),
            );
        }
        DetailTab::Release => {
            ui.label(
                RichText::new("Release form lands in a later step.")
                    .color(ui.style().visuals.weak_text_color()),
            );
        }
    }
}
