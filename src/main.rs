// yImage — fast Windows image viewer & editor.
//
// Entry point. Bootstraps tracing, reads CLI args, and launches the eframe app.
// On Windows the binary is linked as a GUI subsystem app so no console window
// flashes at startup.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// egui 0.34 deprecated a handful of free-standing helpers (SidePanel / close_menu /
// menu::bar / etc.) in favour of the new Panel/UiKind API. Silence those so our
// -Dwarnings CI pass succeeds while we migrate incrementally.
#![allow(deprecated)]
// A few fields (e.g. MosaicState::rect) and helpers exist for symmetry and
// incremental wiring. Don't fail the build on dead_code during bring-up.
#![allow(dead_code)]

use std::path::PathBuf;

mod app;
mod document;
mod i18n;
mod io;
mod ops;
mod registry;
mod tools;
mod ui;

#[cfg(all(windows, feature = "capture"))]
mod capture;

use app::YImageApp;

/// Resolve the directory bundled assets (fonts, ONNX models, icons) live in.
/// Installed builds look beside the executable; development builds fall back
/// to the repo root so `cargo run` works without copying files.
pub fn assets_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let cand = parent.join("assets");
            if cand.exists() {
                return cand;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn main() -> eframe::Result<()> {
    // Initialise logging early so panics and start-up spans are captured.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let startup_file: Option<PathBuf> = std::env::args_os().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("yImage")
            .with_app_id("yimage"),
        vsync: true,
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "yImage",
        native_options,
        Box::new(move |cc| {
            // Install CJK-capable font, theme, and image loaders up-front so the
            // first frame is fully styled.
            ui::setup_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(YImageApp::new(cc, startup_file.clone())))
        }),
    )
}
