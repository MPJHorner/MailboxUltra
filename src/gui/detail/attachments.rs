use egui::RichText;

use crate::message::{Attachment, Message};

use super::DetailContext;

pub fn render(ui: &mut egui::Ui, m: &Message, _ctx: &mut DetailContext<'_>) {
    if m.attachments.is_empty() {
        ui.label(
            RichText::new("(this message has no attachments)")
                .color(ui.style().visuals.weak_text_color()),
        );
        return;
    }
    for (idx, att) in m.attachments.iter().enumerate() {
        ui.push_id(("att", idx), |ui| {
            egui::Frame::group(ui.style())
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| draw_row(ui, att));
        });
        ui.add_space(6.0);
    }
}

fn draw_row(ui: &mut egui::Ui, att: &Attachment) {
    ui.horizontal(|ui| {
        let name = att
            .filename
            .as_deref()
            .unwrap_or("(unnamed attachment)")
            .to_string();
        ui.vertical(|ui| {
            ui.label(RichText::new(&name).strong());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&att.content_type)
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                );
                ui.label(
                    RichText::new(humansize::format_size(att.size as u64, humansize::BINARY))
                        .small()
                        .color(ui.style().visuals.weak_text_color()),
                );
            });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Save…").clicked() {
                save_dialog(att);
            }
            if ui.button("Open").clicked() {
                open_in_default(att);
            }
        });
    });
}

fn save_dialog(att: &Attachment) {
    let mut dialog = rfd::FileDialog::new();
    if let Some(name) = &att.filename {
        dialog = dialog.set_file_name(name);
    }
    if let Some(path) = dialog.save_file() {
        if let Err(e) = std::fs::write(&path, &att.data) {
            tracing::warn!(path = %path.display(), error = %e, "save attachment failed");
        }
    }
}

fn open_in_default(att: &Attachment) {
    let dir = std::env::temp_dir().join("MailBoxUltra-attachments");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let name = att
        .filename
        .clone()
        .unwrap_or_else(|| format!("attachment-{}", uuid::Uuid::new_v4()));
    let path = dir.join(sanitize_filename(&name));
    if let Err(e) = std::fs::write(&path, &att.data) {
        tracing::warn!(path = %path.display(), error = %e, "write temp attachment failed");
        return;
    }
    let _ = open::that(&path);
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
