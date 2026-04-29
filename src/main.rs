//! MailBox Ultra entry point.
//!
//! Exempt from coverage: this file is the eframe boot shim plus the tokio
//! runtime spawn, neither of which can be deterministically driven from a unit
//! test runner. The orchestration logic lives in `crate::server` and is fully
//! tested.

use eframe::egui;

fn main() -> eframe::Result<()> {
    init_tracing();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MailBox Ultra")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([720.0, 480.0]),
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "MailBox Ultra",
        options,
        Box::new(|_cc| Ok(Box::new(StubApp))),
    )
}

struct StubApp;

impl eframe::App for StubApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading("MailBox Ultra");
                ui.add_space(12.0);
                ui.label("Native macOS app — under construction.");
            });
        });
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,mailbox_ultra=info"));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}
