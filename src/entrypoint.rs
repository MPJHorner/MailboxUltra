//! # Top-level entrypoint, kept separate so it can be excluded from coverage.
//!
//! Everything in this file talks to something a unit test runner cannot
//! deterministically drive:
//!
//! - [`run`] orchestrates the whole process and blocks on a Ctrl+C signal.
//! - [`wait_for_shutdown`] reads OS signals via [`tokio::signal::ctrl_c`].
//! - [`spawn_update_check`] hits the GitHub releases API.
//! - [`open_browser`] shells out to `open` / `xdg-open` / `cmd /C start`.
//!
//! The pure orchestration that *can* be tested (binding listeners, building
//! configs, the printer task, the log writer task, `Running` lifecycle) lives
//! in [`crate::app`] and is fully exercised by the unit + integration tests.
//!
//! `codecov.yml`, `.github/workflows/ci.yml`, and `Makefile` all add
//! `src/entrypoint.rs` to their `--ignore-filename-regex` so that running
//! coverage does not show a false 0% on these branches.

use anyhow::Result;
use tokio::signal;
use tokio::task::JoinHandle;

use crate::app::{self, Running};
use crate::cli::Cli;
use crate::output::{Printer, PrinterOptions};
use crate::update;

pub async fn run(cli: Cli) -> Result<()> {
    let printer = Printer::new(PrinterOptions::from_cli(cli.no_cli, cli.json, cli.verbose));
    let running = app::start(&cli, printer.clone()).await?;
    let update_check = if cli.no_update_check {
        None
    } else {
        Some(spawn_update_check(printer))
    };
    let result = wait_for_shutdown(running).await;
    if let Some(handle) = update_check {
        handle.abort();
    }
    result
}

pub(crate) fn spawn_update_check(printer: Printer) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Some(latest) = update::check_latest_version().await {
            printer.print_update_available(update::current_version(), &latest);
        }
    })
}

async fn wait_for_shutdown(mut running: Running) -> Result<()> {
    let ui_task = running.ui_task.take();
    tokio::select! {
        _ = signal::ctrl_c() => {}
        res = &mut running.smtp_task => {
            if let Ok(Err(e)) = res {
                eprintln!("SMTP server stopped: {e}");
            }
        }
        _ = async {
            match ui_task {
                Some(t) => { let _ = t.await; }
                None => std::future::pending::<()>().await,
            }
        } => {}
    }
    running.smtp_task.abort();
    if let Some(t) = running.printer_task {
        t.abort();
    }
    if let Some(t) = running.log_task {
        t.abort();
    }
    if let Some(t) = running.relay_task {
        t.abort();
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let prog = "open";
    #[cfg(target_os = "linux")]
    let prog = "xdg-open";
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let prog = "xdg-open";
    std::process::Command::new(prog)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "windows")]
pub(crate) fn open_browser(url: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", url])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::PrinterOptions;
    use clap::Parser;

    fn cli_for(args: &[&str]) -> Cli {
        let mut v = vec!["mailbox-ultra"];
        v.extend_from_slice(args);
        Cli::parse_from(v)
    }

    fn quiet_printer() -> Printer {
        Printer::new(PrinterOptions {
            use_color: false,
            json_mode: false,
            verbose: false,
            quiet: true,
        })
    }

    #[tokio::test]
    async fn wait_for_shutdown_returns_when_smtp_task_finishes() {
        let c = cli_for(&["-s", "0", "-u", "0"]);
        let running = app::start(&c, quiet_printer()).await.unwrap();
        running.smtp_task.abort();
        let mock = Running {
            store: running.store,
            smtp_addr: running.smtp_addr,
            ui_addr: running.ui_addr,
            smtp_task: tokio::spawn(async { Ok::<(), anyhow::Error>(()) }),
            ui_task: running.ui_task,
            printer_task: running.printer_task,
            log_task: running.log_task,
            relay_task: running.relay_task,
            relay_switch: running.relay_switch,
        };
        let res =
            tokio::time::timeout(std::time::Duration::from_secs(2), wait_for_shutdown(mock)).await;
        assert!(res.is_ok(), "wait_for_shutdown didn't return in time");
    }
}
