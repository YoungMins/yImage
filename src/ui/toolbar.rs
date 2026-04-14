// Top toolbar. Holds global actions (open / save / capture / GIF builder) and
// tool selectors (draw, mosaic, background-remove, object-remove).

use crate::app::{BgMsg, YImageApp};
use crate::tools::ToolKind;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button(app.i18n.t("menu-file", &[]), |ui| {
                if ui.button(app.i18n.t("action-open", &[])).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter(
                            "images",
                            &[
                                "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff",
                                "avif",
                            ],
                        )
                        .pick_file()
                    {
                        app.open_path(&p);
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        app.doc.is_some(),
                        egui::Button::new(app.i18n.t("action-save-as", &[])),
                    )
                    .clicked()
                {
                    app.dialog.save_dialog_open = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        app.doc.is_some(),
                        egui::Button::new(app.i18n.t("action-optimize", &[])),
                    )
                    .clicked()
                {
                    app.dialog.optimize_open = true;
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        app.doc.is_some(),
                        egui::Button::new(app.i18n.t("action-resize", &[])),
                    )
                    .clicked()
                {
                    app.dialog.resize_open = true;
                    if let Some(doc) = &app.doc {
                        app.dialog.resize_w = doc.width();
                        app.dialog.resize_h = doc.height();
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        app.doc.is_some(),
                        egui::Button::new(app.i18n.t("action-convert", &[])),
                    )
                    .clicked()
                {
                    app.dialog.convert_open = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(app.i18n.t("action-gif", &[])).clicked() {
                    app.dialog.gif_open = true;
                    ui.close_menu();
                }
                #[cfg(all(windows, feature = "capture"))]
                if ui.button(app.i18n.t("action-capture", &[])).clicked() {
                    trigger_capture(app);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(app.i18n.t("action-set-default", &[])).clicked() {
                    match crate::registry::register_file_associations() {
                        Ok(_) => {
                            let _ = app
                                .tx
                                .send(BgMsg::Info(app.i18n.t("status-default-ok", &[])));
                        }
                        Err(e) => {
                            let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(app.i18n.t("action-quit", &[])).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button(app.i18n.t("menu-edit", &[]), |ui| {
                if ui.button(app.i18n.t("action-undo", &[])).clicked() {
                    if let Some(doc) = app.doc.as_mut() {
                        if doc.undo() {
                            app.texture_dirty = true;
                        }
                    }
                    ui.close_menu();
                }
                if ui.button(app.i18n.t("action-redo", &[])).clicked() {
                    if let Some(doc) = app.doc.as_mut() {
                        if doc.redo() {
                            app.texture_dirty = true;
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button(app.i18n.t("menu-view", &[]), |ui| {
                if ui.button(app.i18n.t("action-fit", &[])).clicked() {
                    app.viewer.reset_view = true;
                    ui.close_menu();
                }
                if ui.button(app.i18n.t("action-zoom-100", &[])).clicked() {
                    app.viewer.zoom = 1.0;
                    ui.close_menu();
                }
            });

            ui.menu_button(app.i18n.t("menu-lang", &[]), |ui| {
                for lang in ["en-US", "ko-KR", "ja-JP"] {
                    if ui.button(lang).clicked() {
                        app.settings.language = lang.to_string();
                        app.i18n = crate::i18n::I18n::new(lang);
                        ui.close_menu();
                    }
                }
            });

            ui.separator();

            ui.selectable_value(&mut app.tool, ToolKind::None, app.i18n.t("tool-none", &[]));
            ui.selectable_value(&mut app.tool, ToolKind::Draw, app.i18n.t("tool-draw", &[]));
            ui.selectable_value(
                &mut app.tool,
                ToolKind::Mosaic,
                app.i18n.t("tool-mosaic", &[]),
            );
            ui.selectable_value(
                &mut app.tool,
                ToolKind::BackgroundRemove,
                app.i18n.t("tool-bg-remove", &[]),
            );
            ui.selectable_value(
                &mut app.tool,
                ToolKind::ObjectRemove,
                app.i18n.t("tool-obj-remove", &[]),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to("\u{2764} Ko-fi", "https://ko-fi.com/youngminkim");
            });
        });
    });
}

#[cfg(all(windows, feature = "capture"))]
fn trigger_capture(app: &mut YImageApp) {
    let tx = app.tx.clone();
    std::thread::spawn(move || match crate::capture::capture_primary_screen() {
        Ok(img) => {
            let path = std::env::temp_dir().join(format!(
                "yimage-capture-{}.png",
                chrono_stub_now()
            ));
            if let Err(e) = crate::io::save::save_image(&img, &path) {
                let _ = tx.send(BgMsg::Error(format!("save capture: {e:#}")));
                return;
            }
            let _ = tx.send(BgMsg::ImageLoaded { path, image: img });
        }
        Err(e) => {
            let _ = tx.send(BgMsg::Error(format!("capture: {e:#}")));
        }
    });
}

#[cfg(all(windows, feature = "capture"))]
fn chrono_stub_now() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
