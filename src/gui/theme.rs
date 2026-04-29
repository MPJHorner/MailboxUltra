//! Visual tokens. Light + dark palettes inspired by the brand teal-emerald
//! gradient: subtle surface chrome, bold accent for selection / primary
//! buttons.

use egui::{Color32, CornerRadius, Stroke, Visuals};

use crate::settings::Theme;

const ACCENT_LIGHT: Color32 = Color32::from_rgb(45, 212, 191); // teal-400
const ACCENT_DARK: Color32 = Color32::from_rgb(20, 184, 166); // teal-500
const ACCENT_PRESSED: Color32 = Color32::from_rgb(13, 148, 136); // teal-600
const SURFACE_DARK: Color32 = Color32::from_rgb(15, 23, 32);
const PANEL_DARK: Color32 = Color32::from_rgb(22, 33, 45);
const ELEVATED_DARK: Color32 = Color32::from_rgb(28, 41, 56);

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let visuals = match resolve(ctx, theme) {
        ResolvedTheme::Dark => dark_visuals(),
        ResolvedTheme::Light => light_visuals(),
    };
    ctx.set_visuals(visuals);
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);
    ctx.set_global_style(style);
}

enum ResolvedTheme {
    Dark,
    Light,
}

fn resolve(ctx: &egui::Context, theme: Theme) -> ResolvedTheme {
    match theme {
        Theme::Dark => ResolvedTheme::Dark,
        Theme::Light => ResolvedTheme::Light,
        Theme::System => match ctx.theme() {
            egui::Theme::Light => ResolvedTheme::Light,
            egui::Theme::Dark => ResolvedTheme::Dark,
        },
    }
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.window_fill = SURFACE_DARK;
    v.panel_fill = PANEL_DARK;
    v.extreme_bg_color = SURFACE_DARK;
    v.faint_bg_color = ELEVATED_DARK;
    v.widgets.noninteractive.bg_fill = PANEL_DARK;
    v.widgets.inactive.bg_fill = ELEVATED_DARK;
    v.widgets.inactive.weak_bg_fill = ELEVATED_DARK;
    v.widgets.hovered.bg_fill = Color32::from_rgb(36, 53, 70);
    v.widgets.active.bg_fill = ACCENT_PRESSED;
    v.widgets.active.weak_bg_fill = ACCENT_PRESSED;
    v.widgets.open.bg_fill = ACCENT_DARK;
    v.selection.bg_fill = ACCENT_DARK;
    v.selection.stroke = Stroke::new(1.0, ACCENT_LIGHT);
    v.hyperlink_color = ACCENT_LIGHT;
    v.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);
    v.widgets.active.corner_radius = CornerRadius::same(6);
    v
}

fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.selection.bg_fill = ACCENT_LIGHT.linear_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, ACCENT_DARK);
    v.hyperlink_color = ACCENT_PRESSED;
    v.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);
    v.widgets.active.corner_radius = CornerRadius::same(6);
    v
}

pub fn accent(ctx: &egui::Context) -> Color32 {
    if matches!(ctx.theme(), egui::Theme::Light) {
        ACCENT_DARK
    } else {
        ACCENT_LIGHT
    }
}
