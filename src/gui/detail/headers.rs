use egui::RichText;
use egui_extras::{Column, TableBuilder};

use crate::message::Message;

pub fn render(ui: &mut egui::Ui, m: &Message) {
    if m.headers.is_empty() {
        ui.label(
            RichText::new("(no headers)").color(ui.style().visuals.weak_text_color()),
        );
        return;
    }
    let row_height = 22.0;
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(220.0).at_least(120.0))
        .column(Column::remainder().at_least(180.0))
        .header(24.0, |mut h| {
            h.col(|ui| {
                ui.label(RichText::new("Header").strong());
            });
            h.col(|ui| {
                ui.label(RichText::new("Value").strong());
            });
        })
        .body(|mut body| {
            for (k, v) in &m.headers {
                body.row(row_height, |mut row| {
                    row.col(|ui| {
                        ui.add(
                            egui::Label::new(RichText::new(k).strong().monospace())
                                .selectable(true),
                        );
                    });
                    row.col(|ui| {
                        ui.add(
                            egui::Label::new(RichText::new(v).monospace())
                                .selectable(true)
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        )
                        .on_hover_text(v);
                    });
                });
            }
        });
}
