//! Left pane: virtualised list of captured messages, plus the
//! waiting-for-mail empty state with a ready-to-paste swaks example.

use chrono::{DateTime, Local};
use egui::{Color32, RichText, Sense, Stroke};
use uuid::Uuid;

use crate::gui::theme;
use crate::message::Message;

const ROW_HEIGHT: f32 = 76.0;

#[derive(Default)]
pub struct InboxState {
    pub selected_id: Option<Uuid>,
    pub search_query: String,
    /// When `Some`, scroll the row that owns this id into view on the next
    /// frame. Cleared after one frame so the user's manual scrolling isn't
    /// fought by repeated auto-scrolls.
    pub scroll_to: Option<Uuid>,
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

pub struct InboxRenderContext<'a> {
    pub paused: bool,
    pub smtp_url: &'a str,
    pub on_copy_swaks: &'a mut Option<String>,
}

pub fn render(
    ui: &mut egui::Ui,
    inbox: &mut InboxState,
    snapshot: &[Message],
    rctx: InboxRenderContext<'_>,
) -> InboxAction {
    let filtered: Vec<&Message> = snapshot.iter().filter(|m| inbox.matches(m)).collect();

    if filtered.is_empty() {
        return draw_empty_state(ui, snapshot.is_empty(), rctx);
    }

    let mut action = InboxAction::None;
    let scroll_target = inbox.scroll_to.take();
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_rows(ui, ROW_HEIGHT, filtered.len(), |ui, range| {
            for idx in range {
                let m = filtered[idx];
                let selected = inbox.selected_id == Some(m.id);
                let response = draw_row(ui, m, selected);
                if response.clicked() {
                    inbox.selected_id = Some(m.id);
                    action = InboxAction::Selected(m.id);
                }
                if scroll_target == Some(m.id) {
                    response.scroll_to_me(Some(egui::Align::Center));
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

fn draw_empty_state(
    ui: &mut egui::Ui,
    no_messages: bool,
    rctx: InboxRenderContext<'_>,
) -> InboxAction {
    if !no_messages {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.label(
                RichText::new("No messages match your search")
                    .color(ui.style().visuals.weak_text_color()),
            );
        });
        return InboxAction::None;
    }

    if rctx.paused {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
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
        });
        return InboxAction::None;
    }

    let host_port = rctx
        .smtp_url
        .strip_prefix("smtp://")
        .unwrap_or(rctx.smtp_url);
    let snippet = format!(
        "swaks --to dev@example.com --from app@example.com \\\n  --server {host_port} \\\n  --header \"Subject: Hello from MailBoxUltra\" \\\n  --body \"It works.\""
    );

    ui.vertical_centered(|ui| {
        ui.add_space(36.0);
        draw_envelope_art(ui);
        ui.add_space(14.0);
        ui.label(
            RichText::new("Waiting for mail")
                .size(20.0)
                .color(ui.style().visuals.text_color()),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Send anything to the SMTP port and it'll show up here.")
                .color(ui.style().visuals.weak_text_color()),
        );
        ui.add_space(14.0);
        let max_w = ui.available_width().min(420.0);
        ui.allocate_ui_with_layout(
            egui::vec2(max_w, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::group(ui.style())
                    .fill(ui.style().visuals.faint_bg_color)
                    .stroke(Stroke::new(
                        1.0,
                        ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                    ))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_min_width(max_w - 30.0);
                        ui.add(
                            egui::Label::new(RichText::new(&snippet).monospace().size(11.5))
                                .selectable(true)
                                .wrap_mode(egui::TextWrapMode::Wrap),
                        );
                    });
            },
        );
        ui.add_space(8.0);
        if ui.button("Copy command").clicked() {
            *rctx.on_copy_swaks = Some(snippet);
        }
    });
    InboxAction::None
}

/// Small mint-coloured envelope outline that matches the brand mark.
fn draw_envelope_art(ui: &mut egui::Ui) {
    let size = egui::vec2(72.0, 56.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let stroke = Stroke::new(1.6, theme::accent(ui.ctx()).gamma_multiply(0.55));
    let body = rect.shrink2(egui::vec2(2.0, 6.0));
    let painter = ui.painter();
    painter.rect_stroke(
        body,
        egui::CornerRadius::same(4),
        stroke,
        egui::StrokeKind::Inside,
    );
    let top_left = body.left_top();
    let top_right = body.right_top();
    let mid = egui::pos2(body.center().x, body.center().y + 2.0);
    painter.line_segment([top_left, mid], stroke);
    painter.line_segment([top_right, mid], stroke);
}

fn draw_row(ui: &mut egui::Ui, m: &Message, selected: bool) -> egui::Response {
    let row_size = egui::vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(row_size, Sense::click());

    let visuals = ui.style().visuals.clone();
    let accent = theme::accent(ui.ctx());

    if response.hovered() && !selected {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Background. Selected rows use a soft accent fill; hover state uses the
    // theme's hovered fill at higher opacity than egui's default so the
    // affordance is unmistakable on a busy inbox.
    let bg = if selected {
        accent.gamma_multiply(0.20)
    } else if response.hovered() {
        accent.gamma_multiply(0.06)
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, bg);
    }

    // Accent left border on the active row.
    if selected {
        let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height()));
        ui.painter()
            .rect_filled(bar, egui::CornerRadius::ZERO, accent);
    }

    // Bottom hairline separator between rows (skipped on the active row so
    // the accent fill reads as a contiguous block).
    if !selected {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 8.0, rect.bottom() - 0.5),
                egui::pos2(rect.right() - 8.0, rect.bottom() - 0.5),
            ],
            Stroke::new(0.5, visuals.widgets.noninteractive.bg_stroke.color),
        );
    }

    // Inner content. 16px left when selected (3px bar + 13px gap),
    // 16px otherwise. 14px right padding. 10px vertical breathing room.
    let left_inset = 16.0;
    let inner = rect
        .shrink2(egui::vec2(0.0, 10.0))
        .with_min_x(rect.left() + left_inset)
        .with_max_x(rect.right() - 14.0);
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner));

    let local: DateTime<Local> = m.received_at.with_timezone(&Local);
    let time = local.format("%H:%M:%S").to_string();
    let from = m
        .from
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_else(|| m.envelope_from.clone());
    let to =
        m.to.first()
            .map(|a| a.address.clone())
            .or_else(|| m.envelope_to.first().cloned())
            .unwrap_or_default();
    let to_extra = if m.envelope_to.len() > 1 {
        format!(" +{}", m.envelope_to.len() - 1)
    } else {
        String::new()
    };
    let subject = m
        .subject
        .as_deref()
        .unwrap_or("(no subject)")
        .replace(['\n', '\r'], " ");
    let size = humansize::format_size(m.size as u64, humansize::BINARY);
    let attach_marker = if m.attachments.is_empty() {
        String::new()
    } else {
        format!("📎{}", m.attachments.len())
    };

    // Selected rows keep the regular text color — the accent-soft bg fill
    // already provides plenty of contrast and switching to white reads as
    // wrong on the light theme.
    let primary = visuals.text_color();
    let muted = visuals.weak_text_color();
    let dim = visuals.text_color().gamma_multiply(0.78);

    // Row 1: from (bold) ... time (mono small)
    child_ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new(from).color(primary).strong()).truncate());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(time).monospace().small().color(muted));
        });
    });
    // Row 2: subject (regular)
    child_ui.add(
        egui::Label::new(
            RichText::new(format!("{subject}  {attach_marker}"))
                .color(dim)
                .size(13.0),
        )
        .truncate(),
    );
    // Row 3: to (small mono) ... size (small mono)
    child_ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(
                RichText::new(format!("→ {to}{to_extra}"))
                    .small()
                    .monospace()
                    .color(muted),
            )
            .truncate(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(size).small().monospace().color(muted));
        });
    });

    response
}
