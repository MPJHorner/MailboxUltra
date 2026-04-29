//! Lightweight bottom-right toast notifications.

use std::time::{Duration, Instant};

use egui::{Align, Align2, Color32, Frame, Layout, RichText, Stroke};

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    expires_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Default)]
pub struct ToastList {
    items: Vec<Toast>,
}

impl ToastList {
    pub fn push(&mut self, kind: ToastKind, text: impl Into<String>) {
        let ttl = match kind {
            ToastKind::Error => Duration::from_secs(8),
            _ => Duration::from_secs(3),
        };
        self.items.push(Toast {
            text: text.into(),
            kind,
            expires_at: Instant::now() + ttl,
        });
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Info, text);
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Success, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Error, text);
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.items.retain(|t| t.expires_at > now);
        if self.items.is_empty() {
            return;
        }
        // Schedule a repaint right when the next toast expires so the UI
        // doesn't have to be polled to clear them.
        if let Some(next) = self.items.iter().map(|t| t.expires_at).min() {
            if let Some(d) = next.checked_duration_since(now) {
                ctx.request_repaint_after(d);
            }
        }

        egui::Area::new(egui::Id::new("toast-area"))
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    for toast in &self.items {
                        let (bg, accent) = colors(toast.kind, ctx);
                        Frame::canvas(ui.style())
                            .fill(bg)
                            .stroke(Stroke::new(1.0, accent))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);
                                ui.label(RichText::new(&toast.text).color(text_color(ctx)));
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }
}

fn colors(kind: ToastKind, ctx: &egui::Context) -> (Color32, Color32) {
    let dark = matches!(ctx.theme(), egui::Theme::Dark);
    match (kind, dark) {
        (ToastKind::Success, true) => (
            Color32::from_rgb(20, 36, 36),
            Color32::from_rgb(45, 212, 191),
        ),
        (ToastKind::Success, false) => (
            Color32::from_rgb(220, 252, 244),
            Color32::from_rgb(13, 148, 136),
        ),
        (ToastKind::Error, true) => (
            Color32::from_rgb(48, 22, 22),
            Color32::from_rgb(248, 113, 113),
        ),
        (ToastKind::Error, false) => (
            Color32::from_rgb(254, 226, 226),
            Color32::from_rgb(220, 38, 38),
        ),
        (ToastKind::Info, true) => (
            Color32::from_rgb(28, 41, 56),
            Color32::from_rgb(148, 163, 184),
        ),
        (ToastKind::Info, false) => (
            Color32::from_rgb(241, 245, 249),
            Color32::from_rgb(100, 116, 139),
        ),
    }
}

fn text_color(ctx: &egui::Context) -> Color32 {
    if matches!(ctx.theme(), egui::Theme::Dark) {
        Color32::from_rgb(226, 232, 240)
    } else {
        Color32::from_rgb(30, 41, 59)
    }
}
