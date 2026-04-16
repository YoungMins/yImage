// Top menu bar. Holds only the drop-down menus (File / Edit / View / Lang)
// and the Ko-fi support link. Tool selection lives in the left-hand
// tool-palette panel (toolpanel.rs) to keep this bar uncluttered.

use crate::app::{BgMsg, YImageApp};
use crate::tools::ToolKind;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::top("toolbar")
        .exact_height(32.0)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(app.i18n.t("menu-file", &[]), |ui| {
                    if ui
                        .button(format!("\u{1F4C2}  {}", app.i18n.t("action-open", &[])))
                        .clicked()
                    {
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
                            app.open_path(&p, true);
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            app.has_doc(),
                            egui::Button::new(format!(
                                "\u{1F4BE}  {}",
                                app.i18n.t("action-save", &[])
                            )),
                        )
                        .clicked()
                    {
                        app.save_current();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            app.has_doc(),
                            egui::Button::new(format!(
                                "\u{1F4BE}  {}",
                                app.i18n.t("action-save-as", &[])
                            )),
                        )
                        .clicked()
                    {
                        app.dialog.save_dialog_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            app.has_doc(),
                            egui::Button::new(format!(
                                "\u{26A1}  {}",
                                app.i18n.t("action-optimize", &[])
                            )),
                        )
                        .clicked()
                    {
                        app.dialog.optimize_open = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            app.has_doc(),
                            egui::Button::new(format!(
                                "\u{2194}  {}",
                                app.i18n.t("action-resize", &[])
                            )),
                        )
                        .clicked()
                    {
                        app.dialog.resize_open = true;
                        if let Some(tab) = app.tabs.get(app.active_tab) {
                            app.dialog.resize_w = tab.doc.width();
                            app.dialog.resize_h = tab.doc.height();
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            app.has_doc(),
                            egui::Button::new(format!(
                                "\u{21C4}  {}",
                                app.i18n.t("action-convert", &[])
                            )),
                        )
                        .clicked()
                    {
                        app.dialog.convert_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(format!("\u{1F39E}  {}", app.i18n.t("action-gif", &[])))
                        .clicked()
                    {
                        app.tool = ToolKind::Gif;
                        app.dialog.gif_timeline_open = true;
                        ui.close_menu();
                    }

                    // Capture submenu: fullscreen / window / region / fixed / scroll.
                    #[cfg(all(windows, feature = "capture"))]
                    capture_menu(ui, app);

                    // Windows-only: file association + context menu registration.
                    #[cfg(windows)]
                    {
                        ui.separator();
                        if ui
                            .button(format!(
                                "\u{2B50}  {}",
                                app.i18n.t("action-set-default", &[])
                            ))
                            .clicked()
                        {
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
                        if ui
                            .button(format!(
                                "\u{2795}  {}",
                                app.i18n.t("action-register-context", &[])
                            ))
                            .clicked()
                        {
                            let labels = crate::registry::ContextMenuLabels {
                                root: app.i18n.t("ctx-root", &[]),
                                open: app.i18n.t("ctx-open", &[]),
                                optimize: app.i18n.t("ctx-optimize", &[]),
                                resize: app.i18n.t("ctx-resize", &[]),
                                convert: app.i18n.t("ctx-convert", &[]),
                                bg_remove: app.i18n.t("ctx-bg-remove", &[]),
                                obj_remove: app.i18n.t("ctx-obj-remove", &[]),
                            };
                            match crate::registry::register_context_menu(&labels) {
                                Ok(_) => {
                                    let _ = app
                                        .tx
                                        .send(BgMsg::Info(app.i18n.t("status-context-ok", &[])));
                                }
                                Err(e) => {
                                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                                }
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "\u{2796}  {}",
                                app.i18n.t("action-unregister-context", &[])
                            ))
                            .clicked()
                        {
                            match crate::registry::unregister_context_menu() {
                                Ok(_) => {
                                    let _ = app.tx.send(
                                        BgMsg::Info(app.i18n.t("status-context-removed", &[])),
                                    );
                                }
                                Err(e) => {
                                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                                }
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui
                        .button(format!("\u{23FB}  {}", app.i18n.t("action-quit", &[])))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(app.i18n.t("menu-edit", &[]), |ui| {
                    if ui
                        .button(format!("\u{21B6}  {}", app.i18n.t("action-undo", &[])))
                        .clicked()
                    {
                        if let Some(tab) = app.active_tab_mut() {
                            if tab.doc.undo() {
                                tab.texture_dirty = true;
                            }
                        }
                        ui.close_menu();
                    }
                    if ui
                        .button(format!("\u{21B7}  {}", app.i18n.t("action-redo", &[])))
                        .clicked()
                    {
                        if let Some(tab) = app.active_tab_mut() {
                            if tab.doc.redo() {
                                tab.texture_dirty = true;
                            }
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button(app.i18n.t("menu-view", &[]), |ui| {
                    if ui
                        .button(format!("\u{26F6}  {}", app.i18n.t("action-fit", &[])))
                        .clicked()
                    {
                        if let Some(tab) = app.active_tab_mut() {
                            tab.viewer.reset_view = true;
                        }
                        ui.close_menu();
                    }
                    if ui
                        .button(format!(
                            "\u{1F50D}  {}",
                            app.i18n.t("action-zoom-100", &[])
                        ))
                        .clicked()
                    {
                        if let Some(tab) = app.active_tab_mut() {
                            tab.viewer.zoom = 1.0;
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    let mut show_thumbs = app.thumbs.visible;
                    if ui
                        .checkbox(
                            &mut show_thumbs,
                            format!("\u{2630}  {}", app.i18n.t("view-thumbnails", &[])),
                        )
                        .changed()
                    {
                        app.thumbs.visible = show_thumbs;
                    }
                    ui.separator();
                    let mut dark = app.settings.theme_dark;
                    if ui
                        .checkbox(
                            &mut dark,
                            format!("\u{1F319}  {}", app.i18n.t("view-dark-theme", &[])),
                        )
                        .changed()
                    {
                        app.settings.theme_dark = dark;
                        if dark {
                            crate::ui::theme::apply_dark(ctx);
                        } else {
                            crate::ui::theme::apply_light(ctx);
                        }
                    }
                    #[cfg(all(windows, feature = "capture"))]
                    {
                        ui.separator();
                        if ui
                            .button(format!("\u{2328}  {}", app.i18n.t("menu-hotkeys", &[])))
                            .clicked()
                        {
                            app.dialog.hotkeys_open = true;
                            ui.close_menu();
                        }
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("\u{2764} Ko-fi", "https://ko-fi.com/youngminkim");
                });
            });
        });
}

#[cfg(all(windows, feature = "capture"))]
fn capture_menu(ui: &mut egui::Ui, app: &mut YImageApp) {
    let title = format!("\u{1F4F7}  {}", app.i18n.t("action-capture", &[]));
    ui.menu_button(title, |ui| {
        if ui
            .button(format!("\u{25A3}  {}", app.i18n.t("cap-fullscreen", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::Fullscreen);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F1}  {}", app.i18n.t("cap-window", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::ActiveWindow);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F0}  {}", app.i18n.t("cap-region", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::Region);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F3}  {}", app.i18n.t("cap-fixed", &[])))
            .clicked()
        {
            let mode = match app.dialog.fixed_region {
                Some((x, y, w, h)) => crate::capture::CaptureMode::FixedRegion { x, y, w, h },
                None => crate::capture::CaptureMode::FixedRegion {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            };
            app.trigger_capture(mode);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{21C5}  {}", app.i18n.t("cap-scroll", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::AutoScroll);
            ui.close_menu();
        }
        ui.separator();
        if ui
            .button(format!("\u{2328}  {}", app.i18n.t("menu-hotkeys", &[])))
            .clicked()
        {
            app.dialog.hotkeys_open = true;
            ui.close_menu();
        }
    });
}
