//! Top toolbar: brand, SMTP URL pill, search, capture pause, theme toggle,
//! settings + clear buttons.

use egui::{Color32, Key, RichText, Sense, Stroke};

use super::theme;
use super::toasts::ToastList;
use crate::settings::Theme;

pub struct ToolbarContext<'a> {
    pub smtp_url: &'a str,
    pub message_count: usize,
    pub search_query: &'a mut String,
    pub paused: &'a mut bool,
    pub theme: &'a mut Theme,
    pub toasts: &'a mut ToastList,
    pub focus_search: bool,
}

#[derive(Default, PartialEq, Eq)]
pub struct ToolbarOutput {
    pub clear_clicked: bool,
    pub settings_clicked: bool,
    pub help_clicked: bool,
    pub relay_clicked: bool,
}

pub fn render(ui: &mut egui::Ui, tctx: ToolbarContext<'_>) -> ToolbarOutput {
    let mut out = ToolbarOutput::default();

    ui.horizontal_centered(|ui| {
        ui.add_space(4.0);
        // Brand
        ui.label(RichText::new("✉").size(18.0).color(theme::accent(ui.ctx())));
        ui.label(RichText::new("MailBox").strong());
        ui.label(
            RichText::new("Ultra")
                .strong()
                .color(theme::accent(ui.ctx())),
        );

        ui.add_space(12.0);
        smtp_pill(ui, tctx.smtp_url, tctx.toasts);

        ui.add_space(12.0);
        let search = ui.add(
            egui::TextEdit::singleline(tctx.search_query)
                .hint_text("Filter from / to / subject…")
                .desired_width(220.0),
        );
        if tctx.focus_search {
            search.request_focus();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("Clear").color(Color32::from_rgb(248, 113, 113)))
                .on_hover_text("Discard every captured message (⇧⌘X)")
                .clicked()
            {
                out.clear_clicked = true;
            }
            if ui
                .button(if matches!(tctx.theme, Theme::Light) {
                    "🌙"
                } else {
                    "☀"
                })
                .on_hover_text("Toggle theme (T)")
                .clicked()
            {
                *tctx.theme = next_theme(*tctx.theme);
            }
            if ui
                .button("⚙")
                .on_hover_text("Preferences (⌘,)")
                .clicked()
            {
                out.settings_clicked = true;
            }
            if ui
                .button("?")
                .on_hover_text("Keyboard shortcuts (?)")
                .clicked()
            {
                out.help_clicked = true;
            }
            let pause_label = if *tctx.paused { "▶ Resume" } else { "⏸ Pause" };
            if ui
                .button(pause_label)
                .on_hover_text("Pause / resume capture display (P)")
                .clicked()
            {
                *tctx.paused = !*tctx.paused;
            }
            if ui
                .button("↗ Relay")
                .on_hover_text("Configure upstream relay")
                .clicked()
            {
                out.relay_clicked = true;
            }
            ui.label(
                RichText::new(format!("{} captured", tctx.message_count))
                    .color(ui.style().visuals.weak_text_color()),
            );
        });
    });

    out
}

fn smtp_pill(ui: &mut egui::Ui, url: &str, toasts: &mut ToastList) {
    let frame = egui::Frame::group(ui.style())
        .fill(ui.style().visuals.widgets.inactive.bg_fill)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .stroke(Stroke::NONE);
    let response = frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SMTP")
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                );
                ui.label(RichText::new(url).monospace());
            });
        })
        .response
        .interact(Sense::click());
    if response.clicked() {
        ui.ctx().copy_text(url.to_string());
        toasts.success(format!("Copied {url}"));
    }
    response.on_hover_text("Click to copy");
}

fn next_theme(current: Theme) -> Theme {
    match current {
        Theme::System => Theme::Dark,
        Theme::Dark => Theme::Light,
        Theme::Light => Theme::System,
    }
}

pub fn handle_global_shortcuts(
    ctx: &egui::Context,
    paused: &mut bool,
    theme: &mut Theme,
    on_focus_search: &mut bool,
    on_clear: &mut bool,
    on_help: &mut bool,
    on_settings: &mut bool,
) {
    // Skip shortcuts when a text input is focused so search input isn't
    // hijacked by p/t/?/etc.
    if ctx.memory(|m| m.focused().is_some()) {
        // Still allow Esc to blur, ⌘, to open settings, ⌘⇧X to clear.
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(egui::Id::NULL)));
            }
        });
    } else {
        ctx.input(|i| {
            if i.key_pressed(Key::P) {
                *paused = !*paused;
            }
            if i.key_pressed(Key::T) {
                *theme = next_theme(*theme);
            }
            if i.key_pressed(Key::Slash) {
                *on_focus_search = true;
            }
            if i.key_pressed(Key::Questionmark) {
                *on_help = true;
            }
        });
    }
    ctx.input(|i| {
        if i.modifiers.command && i.key_pressed(Key::Comma) {
            *on_settings = true;
        }
        if i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::X) {
            *on_clear = true;
        }
    });
}
