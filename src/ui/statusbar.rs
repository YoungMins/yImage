// Bottom status bar: image dimensions, zoom level, active tool, and the
// latest status message from background workers.

use crate::app::YImageApp;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if let Some(tab) = app.tabs.get(app.active_tab) {
                    let zoom_pct = tab.viewer.zoom * 100.0;
                    ui.weak(format!("{} × {} px", tab.doc.width(), tab.doc.height()));
                    ui.separator();
                    ui.weak(format!("{zoom_pct:.0}%"));
                    ui.separator();
                }
                if app.status.is_empty() {
                    ui.weak("Ready");
                } else {
                    ui.weak(&app.status);
                }
            });
        });
}
