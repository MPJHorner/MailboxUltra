//! MailBox Ultra entry point.
//!
//! Owns:
//! - the tokio runtime that powers the SMTP server, the relay task, and the
//!   log writer task;
//! - eframe's native window;
//! - the wiring between [`ServerHandle`] and [`MailboxApp`].
//!
//! Exempt from coverage. `ServerHandle` (server.rs) and the GUI modules
//! (gui/) are tested independently; nothing here can be deterministically
//! driven from a unit-test runner.

use std::process::ExitCode;

use eframe::egui;

use mailbox_ultra::{gui::MailboxApp, server::ServerHandle, settings::PersistentSettings};

fn main() -> ExitCode {
    init_tracing();

    let settings = PersistentSettings::load();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    let server = match ServerHandle::start(runtime.handle().clone(), settings.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error starting servers: {e:#}");
            // We could still launch the GUI in a degraded state; for now we
            // fail fast so the user sees the binding error.
            return ExitCode::FAILURE;
        }
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("MailBox Ultra")
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([720.0, 480.0]);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        ..Default::default()
    };

    // Hold a runtime guard active during the eframe event loop so background
    // tasks (smtp server, relay, log writer) keep running. The guard is
    // dropped after run_native returns, which aborts every spawned task and
    // releases the listener cleanly.
    let _enter = runtime.enter();
    let server_for_app = server.clone();
    let result = eframe::run_native(
        "MailBox Ultra",
        options,
        Box::new(move |cc| {
            let app = MailboxApp::new(server_for_app.clone(), cc);
            Ok(Box::new(app))
        }),
    );

    server.shutdown();
    drop(runtime);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("eframe error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Load the window icon embedded in the binary at compile time. Returns
/// `None` if the icon file isn't present (e.g. a fresh checkout that
/// hasn't run `make icon` yet) so the build still succeeds.
fn load_window_icon() -> Option<egui::IconData> {
    static BYTES: &[u8] = include_bytes!("../assets/icon-512.png");
    let img = image::load_from_memory(BYTES).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,mailbox_ultra=info"));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}
