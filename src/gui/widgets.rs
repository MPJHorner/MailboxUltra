//! Reusable widgets — pills, badges, chips, the proper checkbox.
//!
//! egui's defaults read fine on a settings panel but feel undersized in the
//! contexts where MailBox Ultra wants to be glanceable: the inbox header
//! strip, the toolbar, the settings dialog. These widgets do their own
//! painting so we can hit the exact dimensions the brand asks for.

use egui::{Color32, CornerRadius, Margin, Response, RichText, Sense, Stroke, Ui, Vec2};

use super::theme;

/// "From" / "To" / value pill: faint background, 1px border, monospace value
/// inside. Set `accent` to tint the value text (used for the From address);
/// pass `None` for plain body-text colour.
pub fn address_pill(ui: &mut Ui, value: &str, accent: Option<Color32>) -> Response {
    let bg = theme::elev2_bg(ui.ctx());
    let border = theme::border_color(ui.ctx());
    let text_color = accent.unwrap_or_else(|| theme::body_text_color(ui.ctx()));
    let frame = egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(12, 4));
    frame
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .color(text_color)
                        .monospace()
                        .size(13.0),
                )
                .selectable(true),
            );
        })
        .response
}

/// Accented pill — small uppercase label, tinted background, matching border.
/// Used for the AUTH indicator on authenticated messages.
pub fn accent_pill(ui: &mut Ui, label: &str) -> Response {
    let accent = theme::accent(ui.ctx());
    let frame = egui::Frame::new()
        .fill(accent.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, accent))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(8, 2));
    frame
        .show(ui, |ui| {
            ui.label(RichText::new(label).color(accent).strong().small());
        })
        .response
}

/// Square icon button — single glyph, fixed size, tooltip on hover.
pub fn icon_button(ui: &mut Ui, glyph: &str, tooltip: &str) -> Response {
    icon_button_inner(ui, glyph, tooltip, false, None)
}

/// Toggle variant: paints the button as if currently engaged.
pub fn icon_toggle(ui: &mut Ui, glyph: &str, tooltip: &str, active: bool) -> Response {
    icon_button_inner(ui, glyph, tooltip, active, None)
}

/// Coloured-glyph variant: typically used for the destructive "Clear" action.
pub fn icon_button_colored(ui: &mut Ui, glyph: &str, tooltip: &str, color: Color32) -> Response {
    icon_button_inner(ui, glyph, tooltip, false, Some(color))
}

fn icon_button_inner(
    ui: &mut Ui,
    glyph: &str,
    tooltip: &str,
    active: bool,
    color_override: Option<Color32>,
) -> Response {
    let size = Vec2::splat(30.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let bg = if active {
        theme::accent_soft(ui.ctx())
    } else if hover {
        theme::soft_bg(ui.ctx())
    } else {
        Color32::TRANSPARENT
    };
    let border = if active {
        theme::accent(ui.ctx())
    } else if hover {
        theme::border_color(ui.ctx())
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(6), bg);
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, border),
            egui::epaint::StrokeKind::Inside,
        );
    }
    let glyph_color = color_override.unwrap_or(if active {
        theme::accent(ui.ctx())
    } else {
        theme::body_text_color(ui.ctx())
    });
    let font = egui::FontId::proportional(15.0);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        font,
        glyph_color,
    );
    if !tooltip.is_empty() {
        resp = resp.on_hover_text(tooltip);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Properly-visible checkbox: an 18×18 box with a heavy border that fills
/// with the brand accent and shows a white tick when on. Whole row is one
/// click target. Replaces egui's tiny default checkbox in the settings dialog.
pub fn nice_checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let box_size: f32 = 18.0;
    let gap: f32 = 10.0;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let text_color = theme::body_text_color(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_color);
    let h = box_size.max(galley.size().y) + 6.0;
    let w = box_size + gap + galley.size().x + 4.0;

    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }

    let painter = ui.painter_at(rect);
    let box_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 2.0, rect.center().y - box_size / 2.0),
        Vec2::splat(box_size),
    );
    let hovered = resp.hovered();
    let accent = theme::accent(ui.ctx());
    let bg = if *value {
        accent
    } else {
        ui.visuals().extreme_bg_color
    };
    let border = if *value {
        accent
    } else if hovered {
        theme::ACCENT_STRONG
    } else {
        theme::muted_text_color(ui.ctx())
    };
    painter.rect_filled(box_rect, CornerRadius::same(4), bg);
    painter.rect_stroke(
        box_rect,
        CornerRadius::same(4),
        Stroke::new(1.5, border),
        egui::epaint::StrokeKind::Inside,
    );

    if *value {
        let c = box_rect.center();
        let p1 = egui::pos2(c.x - 4.5, c.y);
        let p2 = egui::pos2(c.x - 1.0, c.y + 3.5);
        let p3 = egui::pos2(c.x + 5.0, c.y - 3.5);
        let stroke = Stroke::new(2.0, Color32::WHITE);
        painter.line_segment([p1, p2], stroke);
        painter.line_segment([p2, p3], stroke);
    }

    let label_pos = egui::pos2(
        box_rect.right() + gap,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(label_pos, galley, text_color);

    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Section heading inside a settings-style panel. Uppercase, dim, with a
/// thin underline that runs to the panel's right edge.
pub fn section_heading(ui: &mut Ui, title: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title.to_uppercase())
                .small()
                .strong()
                .color(theme::dim_text_color(ui.ctx())),
        );
    });
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(r.left(), r.top() + 1.0),
            egui::pos2(r.right(), r.top() + 1.0),
        ],
        Stroke::new(1.0, theme::border_color(ui.ctx())),
    );
    ui.add_space(8.0);
}

/// Small status dot — used for the relay-active indicator on the toolbar.
pub fn status_dot(ui: &mut Ui, ok: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    let color = if ok { theme::SUCCESS } else { theme::DANGER };
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

#[cfg(test)]
mod tests {
    // Widgets paint into a real egui frame and aren't easily unit-tested
    // outside a runtime. Visual smoke tests cover them.
}
