// Right-hand side panel with parameters for the currently-active tool.
// When no tool is selected we show document info instead.

use crate::app::YImageApp;
use crate::tools::ToolKind;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::SidePanel::right("sidebar")
        .resizable(true)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.heading(app.i18n.t("sidebar-title", &[]));
            ui.separator();

            match app.tool {
                ToolKind::Draw => {
                    ui.label(app.i18n.t("tool-draw", &[]));
                    ui.add(
                        egui::Slider::new(&mut app.dialog.brush.radius, 1.0..=128.0)
                            .text(app.i18n.t("brush-size", &[])),
                    );
                    ui.add(
                        egui::Slider::new(&mut app.dialog.brush.hardness, 0.0..=1.0)
                            .text(app.i18n.t("brush-hardness", &[])),
                    );
                    let mut color = egui::Color32::from_rgba_unmultiplied(
                        app.dialog.brush.color[0],
                        app.dialog.brush.color[1],
                        app.dialog.brush.color[2],
                        app.dialog.brush.color[3],
                    );
                    if egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut color,
                        egui::color_picker::Alpha::Opaque,
                    )
                    .changed()
                    {
                        app.dialog.brush.color = [color.r(), color.g(), color.b(), color.a()];
                    }
                }
                ToolKind::Mosaic => {
                    ui.label(app.i18n.t("tool-mosaic", &[]));
                    ui.add(
                        egui::Slider::new(&mut app.dialog.mosaic.block_size, 2..=128)
                            .text(app.i18n.t("mosaic-block-size", &[])),
                    );
                    ui.label(app.i18n.t("mosaic-hint", &[]));
                }
                ToolKind::BackgroundRemove => {
                    ui.label(app.i18n.t("tool-bg-remove", &[]));
                    ui.label(app.i18n.t("bg-remove-hint", &[]));
                    if ui
                        .add_enabled(
                            app.doc.is_some(),
                            egui::Button::new(app.i18n.t("action-run", &[])),
                        )
                        .clicked()
                    {
                        run_bg_remove(app);
                    }
                }
                ToolKind::ObjectRemove => {
                    ui.label(app.i18n.t("tool-obj-remove", &[]));
                    ui.label(app.i18n.t("obj-remove-hint", &[]));
                    if ui.button(app.i18n.t("action-clear-mask", &[])).clicked() {
                        app.dialog.obj_mask = None;
                    }
                    if ui
                        .add_enabled(
                            app.doc.is_some() && app.dialog.obj_mask.is_some(),
                            egui::Button::new(app.i18n.t("action-run", &[])),
                        )
                        .clicked()
                    {
                        run_obj_remove(app);
                    }
                }
                ToolKind::None => {
                    if let Some(doc) = &app.doc {
                        ui.label(format!(
                            "{} × {} px",
                            doc.width(),
                            doc.height()
                        ));
                        if let Some(path) = &doc.path {
                            ui.label(path.display().to_string());
                        }
                    } else {
                        ui.label(app.i18n.t("sidebar-empty", &[]));
                    }
                }
            }

            ui.separator();
            if let Some((label, v)) = &app.progress {
                ui.label(label);
                ui.add(egui::ProgressBar::new(*v));
            }
        });
}

fn run_bg_remove(app: &mut YImageApp) {
    let Some(doc) = app.doc.as_ref() else { return };
    let image = doc.image.clone();
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::app::BgMsg::Progress(
            "background removal".into(),
            0.1,
        ));
        match crate::tools::bg_remove::remove_background(&image) {
            Ok(out) => {
                let _ = tx.send(crate::app::BgMsg::ImageLoaded {
                    path: std::env::temp_dir().join("yimage-bg-removed.png"),
                    image: out,
                });
            }
            Err(e) => {
                let _ = tx.send(crate::app::BgMsg::Error(format!("{e:#}")));
            }
        }
    });
}

fn run_obj_remove(app: &mut YImageApp) {
    let Some(doc) = app.doc.as_ref() else { return };
    let Some(mask) = app.dialog.obj_mask.clone() else {
        return;
    };
    let image = doc.image.clone();
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::app::BgMsg::Progress("inpaint".into(), 0.1));
        match crate::tools::obj_remove::inpaint(&image, &mask) {
            Ok(out) => {
                let _ = tx.send(crate::app::BgMsg::ImageLoaded {
                    path: std::env::temp_dir().join("yimage-inpainted.png"),
                    image: out,
                });
            }
            Err(e) => {
                let _ = tx.send(crate::app::BgMsg::Error(format!("{e:#}")));
            }
        }
    });
}
