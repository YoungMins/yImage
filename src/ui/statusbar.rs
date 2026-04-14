// Bottom status bar: current tool label, image dimensions, zoom level, and
// the latest status message from background workers.

use crate::app::YImageApp;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(doc) = &app.doc {
                ui.label(format!("{} × {}", doc.width(), doc.height()));
                ui.separator();
                ui.label(format!("{:.0}%", app.viewer.zoom * 100.0));
                ui.separator();
            }
            ui.label(&app.status);
        });
    });
}
