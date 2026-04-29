//! Left pane: virtualised list of captured messages, plus the
//! waiting-for-mail empty state with a ready-to-paste swaks example.
//!
//! Row design — two lines, designed to fit a narrow column without wrapping:
//!
//! ```text
//! ┃ alice.chen@example.com         2m ago
//! ┃ Re: Sprint review timing — what about…   📎2
//! ```
//!
//! Row 1: from address (strong, ellipsis) — relative time (small, muted).
//! Row 2: subject (dim, ellipsis)         — attachment marker (accent), if any.
//!
//! `humanize_relative` collapses the timestamp to "now / Xs / Xm / Xh / Xd" so
//! the right column stays narrow and we don't burn space on a clock-style
//! HH:MM:SS that the user can already see in the detail pane.

use chrono::{DateTime, Local, Utc};
use egui::{Color32, RichText, Sense, Stroke};
use uuid::Uuid;

use crate::gui::theme;
use crate::message::Message;

const ROW_HEIGHT: f32 = 62.0;

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
    let now = Utc::now();
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_rows(ui, ROW_HEIGHT, filtered.len(), |ui, range| {
            for idx in range {
                let m = filtered[idx];
                let selected = inbox.selected_id == Some(m.id);
                let response = draw_row(ui, m, selected, now);
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
                    .color(theme::muted_text_color(ui.ctx())),
            );
        });
        return InboxAction::None;
    }

    if rctx.paused {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.label(
                RichText::new("Capture display paused")
                    .color(theme::body_text_color(ui.ctx()))
                    .size(16.0),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Press P or click Resume to continue")
                    .color(theme::muted_text_color(ui.ctx())),
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
                .color(theme::body_text_color(ui.ctx())),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Send anything to the SMTP port and it'll show up here.")
                .color(theme::muted_text_color(ui.ctx())),
        );
        ui.add_space(14.0);
        let max_w = ui.available_width().min(420.0);
        ui.allocate_ui_with_layout(
            egui::vec2(max_w, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::group(ui.style())
                    .fill(theme::elev2_bg(ui.ctx()))
                    .stroke(Stroke::new(1.0, theme::border_color(ui.ctx())))
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

fn draw_row(ui: &mut egui::Ui, m: &Message, selected: bool, now: DateTime<Utc>) -> egui::Response {
    let row_size = egui::vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(row_size, Sense::click());

    let accent = theme::accent(ui.ctx());
    let hovered = response.hovered();

    if hovered && !selected {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Background. Selected → soft accent fill. Hover → elev2.
    let bg = if selected {
        accent.gamma_multiply(0.18)
    } else if hovered {
        theme::elev2_bg(ui.ctx())
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, bg);
    }

    // Left edge accent bar — solid 3px on selected, faint 2px hint on hover.
    if selected {
        let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height()));
        ui.painter()
            .rect_filled(bar, egui::CornerRadius::ZERO, accent);
    } else if hovered {
        let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
        ui.painter()
            .rect_filled(bar, egui::CornerRadius::ZERO, accent.gamma_multiply(0.55));
    }

    // Bottom hairline separator (skipped when selected so the fill is contiguous).
    if !selected {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 14.0, rect.bottom() - 0.5),
                egui::pos2(rect.right() - 14.0, rect.bottom() - 0.5),
            ],
            Stroke::new(0.5, theme::border_color(ui.ctx())),
        );
    }

    // Inner content rect.
    let left_inset = 16.0;
    let right_inset = 14.0;
    let v_pad = 10.0;
    let inner = rect
        .shrink2(egui::vec2(0.0, v_pad))
        .with_min_x(rect.left() + left_inset)
        .with_max_x(rect.right() - right_inset);

    let row_h = inner.height() / 2.0;
    let row1 = egui::Rect::from_min_size(inner.min, egui::vec2(inner.width(), row_h));
    let row2 = egui::Rect::from_min_size(
        egui::pos2(inner.min.x, inner.min.y + row_h),
        egui::vec2(inner.width(), row_h),
    );

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
    let rel_time = humanize_relative(m.received_at, now);
    let attach_count = m.attachments.len();

    let from_color = theme::body_text_color(ui.ctx());
    let time_color = theme::muted_text_color(ui.ctx());
    let subject_color = if selected {
        theme::body_text_color(ui.ctx())
    } else {
        theme::dim_text_color(ui.ctx())
    };
    let attach_color = accent.gamma_multiply(0.9);

    // Right-side widgets first — relative time on row 1, attach badge on row 2.
    // Paint them via the parent's painter (clipped to the row's rect) so we
    // can compute the leftmost x they occupy and use the rest for the
    // truncated from / subject labels.
    let painter = ui.painter_at(rect);

    let time_galley = painter.layout_no_wrap(
        rel_time,
        egui::TextStyle::Small.resolve(ui.style()),
        time_color,
    );
    let time_size = time_galley.size();
    let time_pos = egui::pos2(
        row1.right() - time_size.x,
        row1.center().y - time_size.y / 2.0,
    );
    painter.galley(time_pos, time_galley, time_color);
    let row1_right_edge = time_pos.x - 8.0;

    let row2_right_edge = if attach_count > 0 {
        let attach_text = format!("📎 {attach_count}");
        let g = painter.layout_no_wrap(
            attach_text,
            egui::TextStyle::Small.resolve(ui.style()),
            attach_color,
        );
        let pos = egui::pos2(
            row2.right() - g.size().x,
            row2.center().y - g.size().y / 2.0,
        );
        let left = pos.x - 8.0;
        painter.galley(pos, g, attach_color);
        left
    } else {
        row2.right()
    };

    // Left side — from + subject, both with ellipsis truncation in their
    // remaining horizontal slot.
    let from_clip = egui::Rect::from_min_max(row1.min, egui::pos2(row1_right_edge, row1.max.y));
    let subject_clip = egui::Rect::from_min_max(row2.min, egui::pos2(row2_right_edge, row2.max.y));

    {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(from_clip)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.add(
            egui::Label::new(RichText::new(from).color(from_color).strong().size(13.5)).truncate(),
        );
    }
    {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(subject_clip)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.add(
            egui::Label::new(RichText::new(subject).color(subject_color).size(12.5)).truncate(),
        );
    }

    response
}

/// Compact relative-time formatter — "now / Xs / Xm / Xh / Xd ago".
///
/// Designed for an inbox row's right-hand timestamp where horizontal space is
/// scarce and absolute precision is unhelpful (the detail pane shows the full
/// timestamp). Anything within 5s collapses to "now" so the label doesn't
/// flicker every render. Future timestamps (clock skew) also collapse to "now".
pub fn humanize_relative(received_at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let _local: DateTime<Local> = received_at.with_timezone(&Local);
    let diff = now.signed_duration_since(received_at);
    let secs = diff.num_seconds();
    if secs < 5 {
        return "now".into();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = diff.num_minutes();
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = diff.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = diff.num_days();
    format!("{days}d ago")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn humanize_relative_buckets() {
        let now = Utc::now();
        assert_eq!(humanize_relative(now, now), "now");
        assert_eq!(humanize_relative(now - Duration::seconds(2), now), "now");
        assert_eq!(
            humanize_relative(now - Duration::seconds(12), now),
            "12s ago"
        );
        assert_eq!(
            humanize_relative(now - Duration::seconds(60), now),
            "1m ago"
        );
        assert_eq!(
            humanize_relative(now - Duration::minutes(45), now),
            "45m ago"
        );
        assert_eq!(humanize_relative(now - Duration::hours(1), now), "1h ago");
        assert_eq!(humanize_relative(now - Duration::hours(23), now), "23h ago");
        assert_eq!(humanize_relative(now - Duration::days(1), now), "1d ago");
        assert_eq!(humanize_relative(now - Duration::days(30), now), "30d ago");
    }

    #[test]
    fn humanize_relative_handles_clock_skew_into_future() {
        let now = Utc::now();
        assert_eq!(humanize_relative(now + Duration::seconds(2), now), "now");
    }
}
