//! Left pane: virtualised list of captured messages.

use chrono::{DateTime, Local};
use egui::{Color32, RichText, Sense, Stroke};
use uuid::Uuid;

use crate::message::Message;

const ROW_HEIGHT: f32 = 64.0;

#[derive(Default)]
pub struct InboxState {
    pub selected_id: Option<Uuid>,
    pub search_query: String,
}

impl InboxState {
    pub fn matches(&self, m: &Message) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        let needle = self.search_query.to_ascii_lowercase();
        let from = m
            .from
            .as_ref()
            .map(|a| a.address.as_str())
            .unwrap_or(m.envelope_from.as_str());
        let to_first =
            m.to.first()
                .map(|a| a.address.as_str())
                .or_else(|| m.envelope_to.first().map(|s| s.as_str()))
                .unwrap_or("");
        let subject = m.subject.as_deref().unwrap_or("");
        from.to_ascii_lowercase().contains(&needle)
            || to_first.to_ascii_lowercase().contains(&needle)
            || subject.to_ascii_lowercase().contains(&needle)
    }
}

pub fn render(
    ui: &mut egui::Ui,
    inbox: &mut InboxState,
    snapshot: &[Message],
    paused: bool,
) -> InboxAction {
    let filtered: Vec<&Message> = snapshot.iter().filter(|m| inbox.matches(m)).collect();

    if filtered.is_empty() {
        return draw_empty_state(ui, snapshot.is_empty(), paused);
    }

    let mut action = InboxAction::None;
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_rows(ui, ROW_HEIGHT, filtered.len(), |ui, range| {
            for idx in range {
                let m = filtered[idx];
                let selected = inbox.selected_id == Some(m.id);
                if draw_row(ui, m, selected) {
                    inbox.selected_id = Some(m.id);
                    action = InboxAction::Selected(m.id);
                }
            }
        });
    action
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum InboxAction {
    #[default]
    None,
    Selected(Uuid),
}

fn draw_empty_state(ui: &mut egui::Ui, no_messages: bool, paused: bool) -> InboxAction {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        if paused {
            ui.label(
                RichText::new("Capture display paused")
                    .color(Color32::from_rgb(180, 180, 180))
                    .size(16.0),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Press P or click Resume to continue")
                    .color(Color32::from_rgb(140, 140, 140)),
            );
        } else if no_messages {
            ui.label(
                RichText::new("Waiting for mail")
                    .color(Color32::from_rgb(180, 180, 180))
                    .size(18.0),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Send anything to the SMTP port and it'll show up here.")
                    .color(Color32::from_rgb(140, 140, 140)),
            );
        } else {
            ui.label(
                RichText::new("No messages match your search")
                    .color(Color32::from_rgb(180, 180, 180)),
            );
        }
    });
    InboxAction::None
}

fn draw_row(ui: &mut egui::Ui, m: &Message, selected: bool) -> bool {
    let row_size = egui::vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(row_size, Sense::click());

    let visuals = ui.style().visuals.clone();
    let bg = if selected {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.bg_fill
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), bg);
    }
    // Bottom hairline separator between rows.
    let sep_color = if selected {
        Color32::TRANSPARENT
    } else {
        visuals.widgets.noninteractive.bg_stroke.color
    };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 8.0, rect.bottom()),
            egui::pos2(rect.right() - 8.0, rect.bottom()),
        ],
        Stroke::new(0.5, sep_color),
    );

    let inner = rect.shrink2(egui::vec2(12.0, 8.0));
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner));

    let local: DateTime<Local> = m.received_at.with_timezone(&Local);
    let time = local.format("%H:%M:%S").to_string();
    let from = m
        .from
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_else(|| m.envelope_from.clone());
    let subject = m
        .subject
        .as_deref()
        .unwrap_or("(no subject)")
        .replace(['\n', '\r'], " ");
    let size = humansize::format_size(m.size as u64, humansize::BINARY);
    let attach_marker = if m.attachments.is_empty() {
        String::new()
    } else {
        format!("  📎 {}", m.attachments.len())
    };

    let muted = if selected {
        Color32::from_rgb(220, 230, 240)
    } else {
        visuals.weak_text_color()
    };
    let primary = if selected {
        Color32::WHITE
    } else {
        visuals.text_color()
    };

    child_ui.horizontal(|ui| {
        ui.label(RichText::new(time).monospace().small().color(muted));
        ui.label(RichText::new(from).color(muted).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(size).small().color(muted));
            if !attach_marker.is_empty() {
                ui.label(RichText::new(attach_marker).color(muted));
            }
        });
    });
    child_ui.add_space(2.0);
    child_ui.label(RichText::new(subject).color(primary).size(14.0));

    response.clicked()
}
