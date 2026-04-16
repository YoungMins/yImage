// Bottom status bar: image dimensions, zoom level, active tool, and the
// latest status message from background workers.

use crate::app::YImageApp;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    // Apple-style status bar: slim, neutral, with thin vertical dividers
    // rather than egui's chunky default separators.
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(26.0)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.spacing_mut().item_spacing.x = 10.0;

                if let Some(tab) = app.tabs.get(app.active_tab) {
                    let zoom_pct = tab.viewer.zoom * 100.0;
                    ui.weak(
                        egui::RichText::new(format!(
                            "{} × {} px",
                            tab.doc.width(),
                            tab.doc.height()
                        ))
                        .size(11.5),
                    );
                    dot_divider(ui);
                    ui.weak(egui::RichText::new(format!("{zoom_pct:.0}%")).size(11.5));
                    dot_divider(ui);
                }
                let status = if app.status.is_empty() {
                    "Ready".to_string()
                } else {
                    app.status.clone()
                };
                ui.weak(egui::RichText::new(status).size(11.5));
            });
        });
}

fn dot_divider(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("·")
            .size(11.5)
            .color(ui.visuals().weak_text_color().linear_multiply(0.7)),
    );
}
