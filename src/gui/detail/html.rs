//! HTML tab — interim implementation.
//!
//! Phase 6 replaces this with a native WKWebView embedded in the eframe
//! window. Until then we show the HTML source highlighted, plus the plain
//! text part if it exists, plus an "Open in browser" escape hatch so
//! users can still see a rendered preview.

use std::path::PathBuf;

use egui::RichText;
use egui_extras::syntax_highlighting::{self, CodeTheme};

use crate::message::Message;

pub fn render(ui: &mut egui::Ui, m: &Message) {
    let Some(html) = m.html.as_ref() else {
        if let Some(text) = &m.text {
            ui.label(
                RichText::new("This message has no HTML part. Showing text/plain.")
                    .small()
                    .color(ui.style().visuals.weak_text_color()),
            );
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::multiline(&mut text.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(28)
                    .code_editor()
                    .interactive(false),
            );
        } else {
            ui.label(
                RichText::new("(no text or HTML body)")
                    .color(ui.style().visuals.weak_text_color()),
            );
        }
        return;
    };

    ui.horizontal(|ui| {
        if ui
            .button("Open in browser")
            .on_hover_text("Write the HTML to a temp file and hand it to the system browser")
            .clicked()
        {
            match write_to_temp(m.id, html) {
                Ok(path) => {
                    let _ = open::that(&path);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to write HTML preview to temp");
                }
            }
        }
        ui.label(
            RichText::new("Native in-window rendering arrives in the next step.")
                .small()
                .color(ui.style().visuals.weak_text_color()),
        );
    });
    ui.add_space(6.0);

    let theme = CodeTheme::from_style(ui.style());
    syntax_highlighting::code_view_ui(ui, &theme, html, "html");
}

fn write_to_temp(id: uuid::Uuid, html: &str) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join("MailBoxUltra-preview");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.html"));
    std::fs::write(&path, html.as_bytes())?;
    Ok(path)
}
