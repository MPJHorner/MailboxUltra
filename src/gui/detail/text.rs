use egui::RichText;

use crate::message::Message;

pub fn render(ui: &mut egui::Ui, m: &Message) {
    match &m.text {
        Some(body) => {
            ui.add(
                egui::TextEdit::multiline(&mut body.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(28)
                    .code_editor()
                    .interactive(false),
            );
        }
        None => {
            ui.label(
                RichText::new("(this message has no text/plain part)")
                    .color(ui.style().visuals.weak_text_color()),
            );
        }
    }
}
