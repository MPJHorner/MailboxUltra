use egui::RichText;
use egui_extras::syntax_highlighting::{self, CodeTheme};

use crate::message::Message;

pub fn render(ui: &mut egui::Ui, m: &Message) {
    let body = String::from_utf8_lossy(&m.raw).into_owned();
    ui.label(
        RichText::new("Raw RFC 822 source")
            .small()
            .color(ui.style().visuals.weak_text_color()),
    );
    ui.add_space(4.0);
    let theme = CodeTheme::from_style(ui.style());
    syntax_highlighting::code_view_ui(ui, &theme, &body, "txt");
}
