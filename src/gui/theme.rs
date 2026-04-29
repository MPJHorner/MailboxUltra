//! Visual tokens. Light + dark palettes inspired by the brand teal-emerald
//! gradient: subtle surface chrome, bold accent for selection / primary
//! buttons. Sets the egui type ramp so SF Pro reads at the same density as
//! the original web UI.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

use crate::settings::Theme;

const ACCENT_LIGHT: Color32 = Color32::from_rgb(45, 212, 191); // teal-400
const ACCENT_DARK: Color32 = Color32::from_rgb(20, 184, 166); // teal-500
const ACCENT_PRESSED: Color32 = Color32::from_rgb(13, 148, 136); // teal-600

// Dark palette — approximates the original web UI's
//   --bg #0a0d11 / --bg-elev #11161c / --bg-elev-2 #161c24 / --bg-soft #1a212a
const D_BG: Color32 = Color32::from_rgb(10, 13, 17);
const D_PANEL: Color32 = Color32::from_rgb(17, 22, 28);
const D_FAINT: Color32 = Color32::from_rgb(22, 28, 36);
const D_ELEV: Color32 = Color32::from_rgb(26, 33, 42);
const D_BORDER: Color32 = Color32::from_rgb(31, 40, 50);
const D_BORDER_STRONG: Color32 = Color32::from_rgb(42, 52, 65);
const D_TEXT: Color32 = Color32::from_rgb(230, 237, 243);
const D_TEXT_DIM: Color32 = Color32::from_rgb(152, 163, 177);
const D_TEXT_MUTE: Color32 = Color32::from_rgb(107, 118, 130);

// Light palette
const L_BG: Color32 = Color32::from_rgb(247, 248, 250);
const L_PANEL: Color32 = Color32::from_rgb(255, 255, 255);
const L_FAINT: Color32 = Color32::from_rgb(241, 243, 246);
const L_ELEV: Color32 = Color32::from_rgb(233, 236, 241);
const L_BORDER: Color32 = Color32::from_rgb(225, 230, 236);
const L_BORDER_STRONG: Color32 = Color32::from_rgb(205, 213, 222);
const L_TEXT: Color32 = Color32::from_rgb(14, 21, 28);
const L_TEXT_DIM: Color32 = Color32::from_rgb(71, 86, 101);
const L_TEXT_MUTE: Color32 = Color32::from_rgb(111, 124, 140);

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let visuals = match resolve(ctx, theme) {
        ResolvedTheme::Dark => dark_visuals(),
        ResolvedTheme::Light => light_visuals(),
    };
    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();

    // Type ramp tuned for SF Pro at standard macOS rendering. Values
    // chosen to match the original web UI: body ~14px, small ~12px.
    use TextStyle::{Body, Button, Heading, Monospace, Small};
    style.text_styles.insert(Heading, FontId::new(22.0, FontFamily::Proportional));
    style.text_styles.insert(Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(Button, FontId::new(13.5, FontFamily::Proportional));
    style.text_styles.insert(Small, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(Monospace, FontId::new(13.0, FontFamily::Monospace));

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(14);
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.indent = 18.0;
    style.visuals.window_corner_radius = CornerRadius::same(12);
    style.visuals.menu_corner_radius = CornerRadius::same(8);

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
    v.window_fill = D_BG;
    v.panel_fill = D_PANEL;
    v.extreme_bg_color = D_BG;
    v.faint_bg_color = D_FAINT;
    v.code_bg_color = D_FAINT;
    // Body text reads from widgets.noninteractive.fg_stroke; weak_text
    // derives from that unless overridden. We set both explicitly so the
    // type ramp has a real strong/weak contrast.
    v.weak_text_color = Some(D_TEXT_MUTE);

    v.widgets.noninteractive.bg_fill = D_PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, D_BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, D_TEXT);
    v.widgets.inactive.bg_fill = D_ELEV;
    v.widgets.inactive.weak_bg_fill = D_ELEV;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, D_BORDER_STRONG);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, D_TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(36, 53, 70);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_LIGHT.gamma_multiply(0.5));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, D_TEXT);
    v.widgets.active.bg_fill = ACCENT_PRESSED;
    v.widgets.active.weak_bg_fill = ACCENT_PRESSED;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_LIGHT);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.open.bg_fill = ACCENT_DARK;
    v.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT_LIGHT);

    v.selection.bg_fill = ACCENT_DARK;
    v.selection.stroke = Stroke::new(1.0, ACCENT_LIGHT);
    v.hyperlink_color = ACCENT_LIGHT;
    let _ = D_TEXT_DIM; // reserved for hand-tinted text where weak_text_color isn't right

    let r = CornerRadius::same(6);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v
}

fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.window_fill = L_BG;
    v.panel_fill = L_PANEL;
    v.extreme_bg_color = L_BG;
    v.faint_bg_color = L_FAINT;
    v.code_bg_color = L_FAINT;
    v.weak_text_color = Some(L_TEXT_MUTE);

    v.widgets.noninteractive.bg_fill = L_PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, L_BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, L_TEXT);
    v.widgets.inactive.bg_fill = L_ELEV;
    v.widgets.inactive.weak_bg_fill = L_ELEV;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, L_BORDER_STRONG);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, L_TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(232, 242, 240);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_DARK.gamma_multiply(0.5));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, L_TEXT);
    v.widgets.active.bg_fill = ACCENT_DARK;
    v.widgets.active.weak_bg_fill = ACCENT_DARK;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_PRESSED);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.open.bg_fill = ACCENT_LIGHT;
    v.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT_DARK);

    v.selection.bg_fill = ACCENT_LIGHT.linear_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, ACCENT_DARK);
    v.hyperlink_color = ACCENT_PRESSED;
    let _ = L_TEXT_DIM;

    let r = CornerRadius::same(6);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v
}

pub fn accent(ctx: &egui::Context) -> Color32 {
    if matches!(ctx.theme(), egui::Theme::Light) {
        ACCENT_DARK
    } else {
        ACCENT_LIGHT
    }
}
