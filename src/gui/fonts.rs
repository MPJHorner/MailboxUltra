//! Install SF Pro as the egui app font on macOS.
//!
//! macOS ships SF Pro at `/System/Library/Fonts/SFNS.ttf` (proportional)
//! and `SFNSMono.ttf` (monospace). They're plain `.ttf` files — readable
//! by any process, no entitlements needed. egui can load them directly
//! into its `FontDefinitions`.
//!
//! On non-mac targets this is a no-op and egui falls back to its
//! built-in Ubuntu fonts.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

const PROPORTIONAL: &str = "system-ui";
const MONOSPACE: &str = "system-mono";

#[cfg(target_os = "macos")]
const PROPORTIONAL_PATHS: &[&str] = &[
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
];
#[cfg(target_os = "macos")]
const MONOSPACE_PATHS: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
];

#[cfg(not(target_os = "macos"))]
const PROPORTIONAL_PATHS: &[&str] = &[];
#[cfg(not(target_os = "macos"))]
const MONOSPACE_PATHS: &[&str] = &[];

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    if let Some(bytes) = first_readable(PROPORTIONAL_PATHS) {
        fonts
            .font_data
            .insert(PROPORTIONAL.to_owned(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, PROPORTIONAL.to_owned());
    }

    if let Some(bytes) = first_readable(MONOSPACE_PATHS) {
        fonts
            .font_data
            .insert(MONOSPACE.to_owned(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, MONOSPACE.to_owned());
    }

    ctx.set_fonts(fonts);
}

fn first_readable(paths: &[&str]) -> Option<Vec<u8>> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            tracing::debug!(path = %path, "loaded font");
            return Some(bytes);
        }
    }
    None
}
