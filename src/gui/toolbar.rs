//! Top toolbar: brand, SMTP URL pill, search, capture pause, theme toggle,
//! settings + clear buttons.

use egui::{Key, RichText, Sense, Stroke};

use super::theme;
use super::toasts::ToastList;
use super::widgets;
use crate::settings::Theme;

pub struct ToolbarContext<'a> {
    pub smtp_url: &'a str,
    pub message_count: usize,
    pub search_query: &'a mut String,
    pub paused: &'a mut bool,
    pub theme: &'a mut Theme,
    pub toasts: &'a mut ToastList,
    pub focus_search: bool,
    pub relay_active: bool,
    pub relay_label: Option<&'a str>,
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
    let accent = theme::accent(ui.ctx());

    // Canonical egui idiom for "left items + right items in a single row":
    // wrap the whole row in `right_to_left`, place right items first (they
    // stack from the right edge), then nest `left_to_right` for the left
    // items (which fills from the left and takes whatever space remains).
    // This is the only pattern in egui that reliably gets both sides.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(14.0);

        // Right cluster — icon buttons read tighter than the default
        // text-button rectangles when stacked five-deep.
        if widgets::icon_button_colored(ui, "🗑", "Clear inbox (⇧⌘X)", theme::DANGER).clicked()
        {
            out.clear_clicked = true;
        }
        if widgets::icon_button(ui, theme_icon(*tctx.theme), "Toggle theme (T)").clicked() {
            *tctx.theme = next_theme(*tctx.theme);
        }
        if widgets::icon_button(ui, "⚙", "Preferences (⌘,)").clicked() {
            out.settings_clicked = true;
        }
        if widgets::icon_button(ui, "?", "Keyboard shortcuts (?)").clicked() {
            out.help_clicked = true;
        }
        let pause_glyph = if *tctx.paused { "▶" } else { "⏸" };
        let pause_tooltip = if *tctx.paused {
            "Resume capture (P)"
        } else {
            "Pause capture (P)"
        };
        if widgets::icon_toggle(ui, pause_glyph, pause_tooltip, *tctx.paused).clicked() {
            *tctx.paused = !*tctx.paused;
        }
        ui.add_space(4.0);
        if relay_button(ui, tctx.relay_active, tctx.relay_label, accent).clicked() {
            out.relay_clicked = true;
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("{} captured", tctx.message_count))
                .small()
                .color(theme::muted_text_color(ui.ctx())),
        );

        // Left cluster (nested left_to_right takes the remaining width).
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(14.0);
            ui.label(RichText::new("✉").size(18.0).color(accent));
            ui.label(
                RichText::new("MailBox Ultra")
                    .strong()
                    .color(theme::body_text_color(ui.ctx())),
            );
            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .monospace()
                    .color(theme::dim_text_color(ui.ctx())),
            );
            ui.add_space(10.0);
            smtp_pill(ui, tctx.smtp_url, tctx.toasts);
            ui.add_space(10.0);
            let search = ui.add(
                egui::TextEdit::singleline(tctx.search_query)
                    .hint_text("Filter from / to / subject  (/)")
                    .desired_width(280.0),
            );
            if tctx.focus_search {
                search.request_focus();
            }
        });
    });

    out
}

fn theme_icon(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "🖥",
        Theme::Dark => "☀",
        Theme::Light => "🌙",
    }
}

/// Pill-style relay button. Off → muted text, on → accent text + matching
/// border + soft accent fill, with the upstream host:port appended in mono.
fn relay_button(
    ui: &mut egui::Ui,
    active: bool,
    label: Option<&str>,
    accent: egui::Color32,
) -> egui::Response {
    let arrow_color = if active {
        accent
    } else {
        theme::muted_text_color(ui.ctx())
    };
    let text_color = if active {
        accent
    } else {
        theme::body_text_color(ui.ctx())
    };
    let label_galley = ui.painter().layout_no_wrap(
        format!("Relay  {}", label.unwrap_or("off")),
        egui::TextStyle::Button.resolve(ui.style()),
        text_color,
    );
    let arrow_galley = ui.painter().layout_no_wrap(
        "↗".to_string(),
        egui::TextStyle::Button.resolve(ui.style()),
        arrow_color,
    );
    let pad_x = 12.0;
    let pad_y = 6.0;
    let inner = label_galley.size().x + 6.0 + arrow_galley.size().x;
    let total = egui::vec2(inner + pad_x * 2.0, label_galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(total, egui::Sense::click());
    let (fill, border) = if active {
        (accent.gamma_multiply(0.18), accent)
    } else if response.hovered() {
        (
            theme::soft_bg(ui.ctx()),
            theme::border_strong_color(ui.ctx()),
        )
    } else {
        (theme::elev2_bg(ui.ctx()), theme::border_color(ui.ctx()))
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let arrow_pos = egui::pos2(
        rect.left() + pad_x,
        rect.center().y - arrow_galley.size().y / 2.0,
    );
    let arrow_size = arrow_galley.size();
    ui.painter().galley(arrow_pos, arrow_galley, arrow_color);
    let label_pos = egui::pos2(
        arrow_pos.x + arrow_size.x + 6.0,
        rect.center().y - label_galley.size().y / 2.0,
    );
    ui.painter().galley(label_pos, label_galley, text_color);
    response
}

fn smtp_pill(ui: &mut egui::Ui, url: &str, toasts: &mut ToastList) {
    // Two-pass render: pick fill from last-frame hover (egui remembers it) so
    // the user gets an immediate hover affordance that it's clickable.
    let id = egui::Id::new("toolbar-smtp-pill");
    let last_hovered = ui
        .ctx()
        .memory(|m| m.data.get_temp::<bool>(id).unwrap_or(false));
    let fill = if last_hovered {
        theme::soft_bg(ui.ctx())
    } else {
        theme::elev2_bg(ui.ctx())
    };
    let frame = egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(12, 5))
        .stroke(Stroke::new(1.0, theme::border_color(ui.ctx())));
    let response = frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SMTP")
                        .small()
                        .strong()
                        .color(theme::dim_text_color(ui.ctx())),
                );
                ui.label(
                    RichText::new(url)
                        .monospace()
                        .color(theme::body_text_color(ui.ctx())),
                );
            });
        })
        .response
        .interact(Sense::click());

    let hovered = response.hovered();
    ui.ctx().memory_mut(|m| m.data.insert_temp(id, hovered));
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
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
