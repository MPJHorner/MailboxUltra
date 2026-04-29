//! HTML tab.
//!
//! On macOS the rendered preview lives inside a real `WKWebView` embedded as
//! a child `NSView` of the eframe window (see `gui::native_html`). egui
//! draws the sub-tab buttons and reserves a rect; the WKWebView is
//! repositioned to overlap that rect each frame.
//!
//! When the message has no HTML body, the tab falls back to plain text.

use std::path::PathBuf;

use egui::RichText;
use egui_extras::syntax_highlighting::{self, CodeTheme};

use crate::message::Message;

use super::DetailContext;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HtmlSubTab {
    #[default]
    Rendered,
    Source,
}

#[derive(Default)]
pub struct HtmlState {
    pub sub_tab: HtmlSubTab,
}

pub fn render(ui: &mut egui::Ui, state: &mut HtmlState, m: &Message, ctx: &mut DetailContext<'_>) {
    let Some(html) = m.html.as_ref() else {
        #[cfg(target_os = "macos")]
        if let Some(view) = ctx.native_html {
            view.set_visible(false);
        }
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
                RichText::new("(no text or HTML body)").color(ui.style().visuals.weak_text_color()),
            );
        }
        return;
    };

    ui.horizontal(|ui| {
        if ui
            .selectable_label(state.sub_tab == HtmlSubTab::Rendered, "Rendered")
            .clicked()
        {
            state.sub_tab = HtmlSubTab::Rendered;
        }
        if ui
            .selectable_label(state.sub_tab == HtmlSubTab::Source, "Source")
            .clicked()
        {
            state.sub_tab = HtmlSubTab::Source;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Open in browser")
                .on_hover_text("Write the HTML to a temp file and shell to the system browser")
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
        });
    });
    ui.add_space(6.0);

    match state.sub_tab {
        HtmlSubTab::Rendered => {
            #[cfg(target_os = "macos")]
            {
                if let Some(view) = ctx.native_html {
                    let rect = ui.available_rect_before_wrap();
                    ui.allocate_rect(rect, egui::Sense::hover());
                    view.set_frame(ctx.window_height, rect);
                    view.set_visible(true);
                    view.load(m.id, html);
                    return;
                }
            }
            ui.label(
                RichText::new(
                    "Native HTML rendering is unavailable on this build. Showing source.",
                )
                .small()
                .color(ui.style().visuals.weak_text_color()),
            );
            render_source(ui, html);
        }
        HtmlSubTab::Source => {
            #[cfg(target_os = "macos")]
            if let Some(view) = ctx.native_html {
                view.set_visible(false);
            }
            render_source(ui, html);
        }
    }
}

fn render_source(ui: &mut egui::Ui, html: &str) {
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
