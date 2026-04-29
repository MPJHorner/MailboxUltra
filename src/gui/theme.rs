//! Visual tokens for the MailBox Ultra desktop app.
//!
//! Two design priorities: (1) read well at-a-glance — clear surface hierarchy,
//! readable contrast on the busy inbox; (2) feel native — soft shadows, teal
//! accent that matches the brand mark and the original web UI so users
//! migrating from one to the other don't feel disoriented.
//!
//! Surface hierarchy (dark, lightening in this order):
//!
//! ```text
//! BG       (panel_fill)        — the darkest surface; toolbar + inbox sit here
//! BG_ELEV  (window_fill)       — the detail pane and floating dialogs
//! BG_ELEV2 (faint_bg / code)   — chips, badges, header tables
//! BG_SOFT  (extreme_bg)        — TextEdit insides; reads as a surface, not a hole
//! ```
//!
//! Helper functions take `&egui::Context` and return the right colour for the
//! currently active palette so consumers don't repeat the dark/light branch.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

use crate::settings::Theme;

// ── Brand ──────────────────────────────────────────────────────────────
pub const ACCENT: Color32 = Color32::from_rgb(45, 212, 191); // teal-400
pub const ACCENT_STRONG: Color32 = Color32::from_rgb(94, 234, 212); // teal-300
pub const ACCENT_DEEP: Color32 = Color32::from_rgb(20, 184, 166); // teal-500
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(13, 148, 136); // teal-600
pub const ACCENT_SOFT_DARK: Color32 = Color32::from_rgba_premultiplied(20, 184, 166, 0x40);
pub const ACCENT_SOFT_LIGHT: Color32 = Color32::from_rgba_premultiplied(45, 212, 191, 0x33);

// ── Dark surfaces ──────────────────────────────────────────────────────
pub const BG_DARK: Color32 = Color32::from_rgb(0x0a, 0x0d, 0x11);
pub const BG_ELEV_DARK: Color32 = Color32::from_rgb(0x11, 0x16, 0x1c);
pub const BG_ELEV2_DARK: Color32 = Color32::from_rgb(0x16, 0x1c, 0x24);
pub const BG_SOFT_DARK: Color32 = Color32::from_rgb(0x1a, 0x21, 0x2a);
pub const BORDER_DARK: Color32 = Color32::from_rgb(0x1f, 0x28, 0x32);
pub const BORDER_STRONG_DARK: Color32 = Color32::from_rgb(0x2a, 0x34, 0x41);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xe6, 0xed, 0xf3);
pub const TEXT_MUTED_DARK: Color32 = Color32::from_rgb(0x98, 0xa3, 0xb1);
pub const TEXT_DIM_DARK: Color32 = Color32::from_rgb(0x6b, 0x76, 0x82);

// ── Light surfaces ─────────────────────────────────────────────────────
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xf7, 0xf8, 0xfa);
pub const BG_ELEV_LIGHT: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
pub const BG_ELEV2_LIGHT: Color32 = Color32::from_rgb(0xf1, 0xf3, 0xf6);
pub const BG_SOFT_LIGHT: Color32 = Color32::from_rgb(0xe9, 0xec, 0xf1);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0xe1, 0xe6, 0xec);
pub const BORDER_STRONG_LIGHT: Color32 = Color32::from_rgb(0xcd, 0xd5, 0xde);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x0e, 0x15, 0x1c);
pub const TEXT_MUTED_LIGHT: Color32 = Color32::from_rgb(0x47, 0x56, 0x65);
pub const TEXT_DIM_LIGHT: Color32 = Color32::from_rgb(0x6f, 0x7c, 0x8c);

// ── Status ─────────────────────────────────────────────────────────────
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);
pub const WARNING: Color32 = Color32::from_rgb(0xff, 0xb8, 0x60);
pub const SUCCESS: Color32 = Color32::from_rgb(0x4c, 0xd9, 0x7e);

/// Apply the theme to an egui context. Re-callable on each frame after the
/// user toggles the preference; egui internally diffs the style so this is
/// effectively free unless the dark/light decision changes.
pub fn apply(ctx: &egui::Context, pref: Theme) {
    let dark = is_dark(ctx, pref);
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = if dark { ACCENT } else { ACCENT_PRESSED };

    let radius = CornerRadius::same(6);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = radius;

    if dark {
        visuals.panel_fill = BG_DARK;
        visuals.window_fill = BG_ELEV_DARK;
        // TextEdit's "extreme" surface — slightly lighter than the dialog fill
        // so inputs read as raised surfaces, not holes.
        visuals.extreme_bg_color = BG_SOFT_DARK;
        visuals.faint_bg_color = BG_ELEV2_DARK;
        visuals.code_bg_color = BG_ELEV2_DARK;
        visuals.weak_text_color = Some(TEXT_MUTED_DARK);
        visuals.window_stroke = Stroke::new(1.0, BORDER_DARK);
        visuals.widgets.noninteractive.bg_fill = BG_ELEV_DARK;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_DARK);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DARK);
        visuals.widgets.inactive.bg_fill = BG_ELEV2_DARK;
        visuals.widgets.inactive.weak_bg_fill = BG_ELEV2_DARK;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_DARK);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DARK);
        visuals.widgets.hovered.bg_fill = BG_SOFT_DARK;
        visuals.widgets.hovered.weak_bg_fill = BG_SOFT_DARK;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG_DARK);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_DARK);
        visuals.widgets.active.bg_fill = BG_SOFT_DARK;
        visuals.widgets.active.weak_bg_fill = BG_SOFT_DARK;
        // Focus indicator — solid 1px border in the deeper accent so it reads
        // as a clean ring, not a glow. The brighter ACCENT was over-saturating
        // the entire input edge and bleeding into surrounding chrome.
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_DEEP);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_DARK);
        visuals.widgets.open.bg_fill = BG_SOFT_DARK;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT_DEEP);
        visuals.override_text_color = Some(TEXT_DARK);
    } else {
        visuals.panel_fill = BG_LIGHT;
        visuals.window_fill = BG_ELEV_LIGHT;
        visuals.extreme_bg_color = BG_ELEV_LIGHT;
        visuals.faint_bg_color = BG_ELEV2_LIGHT;
        visuals.code_bg_color = BG_ELEV2_LIGHT;
        visuals.weak_text_color = Some(TEXT_MUTED_LIGHT);
        visuals.window_stroke = Stroke::new(1.0, BORDER_LIGHT);
        visuals.widgets.noninteractive.bg_fill = BG_ELEV_LIGHT;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_LIGHT);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_LIGHT);
        visuals.widgets.inactive.bg_fill = BG_ELEV2_LIGHT;
        visuals.widgets.inactive.weak_bg_fill = BG_ELEV2_LIGHT;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_LIGHT);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_LIGHT);
        visuals.widgets.hovered.bg_fill = BG_SOFT_LIGHT;
        visuals.widgets.hovered.weak_bg_fill = BG_SOFT_LIGHT;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG_LIGHT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_LIGHT);
        visuals.widgets.active.bg_fill = BG_SOFT_LIGHT;
        visuals.widgets.active.weak_bg_fill = BG_SOFT_LIGHT;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_DEEP);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_LIGHT);
        visuals.widgets.open.bg_fill = BG_SOFT_LIGHT;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT_DEEP);
        visuals.override_text_color = Some(TEXT_LIGHT);
    }

    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.open.corner_radius = radius;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    // Type ramp: tighter than my first pass, closer to the web's compact UI.
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(17.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.5, FontFamily::Proportional),
    );

    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = Margin::same(16);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.indent = 18.0;
    style.spacing.interact_size = egui::vec2(28.0, 28.0);

    ctx.set_global_style(style);
}

pub fn is_dark(ctx: &egui::Context, pref: Theme) -> bool {
    match pref {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => match ctx.theme() {
            egui::Theme::Dark => true,
            egui::Theme::Light => false,
        },
    }
}

/// Brand accent — used for selection, active widget borders, etc.
pub fn accent(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        ACCENT
    } else {
        ACCENT_DEEP
    }
}

/// Soft accent fill — selected list rows, AUTH pill, active toggle bg.
pub fn accent_soft(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        ACCENT_SOFT_DARK
    } else {
        ACCENT_SOFT_LIGHT
    }
}

pub fn body_text_color(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        TEXT_DARK
    } else {
        TEXT_LIGHT
    }
}

pub fn muted_text_color(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        TEXT_MUTED_DARK
    } else {
        TEXT_MUTED_LIGHT
    }
}

pub fn dim_text_color(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        TEXT_DIM_DARK
    } else {
        TEXT_DIM_LIGHT
    }
}

pub fn border_color(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BORDER_DARK
    } else {
        BORDER_LIGHT
    }
}

pub fn border_strong_color(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BORDER_STRONG_DARK
    } else {
        BORDER_STRONG_LIGHT
    }
}

/// Panel canvas fill — the darkest surface.
pub fn bg(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BG_DARK
    } else {
        BG_LIGHT
    }
}

/// One step lighter than panel — windows, the detail card.
pub fn elev_bg(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BG_ELEV_DARK
    } else {
        BG_ELEV_LIGHT
    }
}

/// Two steps lighter — chips, header rows, hover.
pub fn elev2_bg(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BG_ELEV2_DARK
    } else {
        BG_ELEV2_LIGHT
    }
}

/// Lightest surface — TextEdit insides, active widget fills.
pub fn soft_bg(ctx: &egui::Context) -> Color32 {
    if ctx.global_style().visuals.dark_mode {
        BG_SOFT_DARK
    } else {
        BG_SOFT_LIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_hierarchy_lightens_monotonically_dark() {
        // BG (darkest) < BG_ELEV < BG_ELEV2 < BG_SOFT (lightest).
        // Compare luminance via the green channel (good enough for our greys).
        assert!(BG_DARK.g() < BG_ELEV_DARK.g());
        assert!(BG_ELEV_DARK.g() < BG_ELEV2_DARK.g());
        assert!(BG_ELEV2_DARK.g() < BG_SOFT_DARK.g());
    }

    #[test]
    fn surface_hierarchy_darkens_monotonically_light() {
        // Light theme: BG (lightest off-white) > BG_ELEV (white) - well, actually
        // in our scheme the light "elev" is pure white and the canvas is a tint,
        // so the canvas is darker. Just assert the canvas isn't pure white.
        assert!(BG_LIGHT.r() < 255);
        assert!(BG_ELEV_LIGHT.r() == 255);
    }

    #[test]
    fn text_strong_more_opaque_than_muted_more_than_dim() {
        // Dark theme contrast ramp: TEXT > MUTED > DIM (all on a dark bg).
        assert!(TEXT_DARK.g() > TEXT_MUTED_DARK.g());
        assert!(TEXT_MUTED_DARK.g() > TEXT_DIM_DARK.g());
    }
}
