//! Keyboard shortcuts cheat-sheet window.

use egui::RichText;
use egui_extras::{Column, TableBuilder};

#[derive(Default)]
pub struct HelpWindowState {
    pub open: bool,
}

pub fn render(ctx: &egui::Context, state: &mut HelpWindowState) {
    if !state.open {
        return;
    }
    let mut keep_open = true;
    egui::Window::new(RichText::new("Keyboard shortcuts").strong())
        .open(&mut keep_open)
        .resizable(false)
        .collapsible(false)
        .default_size([420.0, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .column(Column::initial(140.0).at_least(120.0))
                .column(Column::remainder().at_least(180.0))
                .body(|mut body| {
                    for (keys, desc) in SHORTCUTS {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(RichText::new(*keys).monospace().strong());
                            });
                            row.col(|ui| {
                                ui.label(*desc);
                            });
                        });
                    }
                });
            ui.add_space(10.0);
            egui::Frame::group(ui.style())
                .fill(ui.style().visuals.faint_bg_color)
                .stroke(egui::Stroke::new(
                    1.0,
                    ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Modifier keys (⌘ / Ctrl / Alt) are never intercepted — ⌘C still copies and ⌘W still closes the window.",
                        )
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                    );
                });
        });
    if !keep_open {
        state.open = false;
    }
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("j / ↓", "Next message"),
    ("k / ↑", "Previous message"),
    ("g", "Jump to newest"),
    ("G (⇧g)", "Jump to oldest"),
    ("/", "Focus search"),
    ("1 – 6", "Switch detail tab"),
    ("p", "Pause / resume capture"),
    ("d", "Delete current message"),
    ("⇧⌘X", "Clear all"),
    ("t", "Toggle theme"),
    ("⌘,", "Preferences"),
    ("?", "Show this help"),
    ("Esc", "Close dialog / blur search"),
];
