// Right-hand Inspector panel.
//
// Shows exhaustive information about the current image (dimensions, aspect,
// colour channels, estimated memory, path, file size + mtime when available),
// plus the parameter block for whichever tool is currently active. The panel
// was previously named "Tool Options"; it is now the home for all document
// metadata so users have a single place to look.

use crate::app::YImageApp;
use crate::tools::ToolKind;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::SidePanel::right("inspector")
        .resizable(true)
        .default_width(280.0)
        .min_width(240.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.heading(app.i18n.t("inspector-title", &[]));
            ui.separator();

            show_image_properties(ui, app);
            ui.separator();
            show_tool_section(ui, app);

            ui.separator();
            if let Some((label, v)) = &app.progress {
                ui.label(label);
                ui.add(egui::ProgressBar::new(*v).show_percentage());
            }
        });
}

fn show_image_properties(ui: &mut egui::Ui, app: &YImageApp) {
    egui::CollapsingHeader::new(app.i18n.t("inspector-properties", &[]))
        .default_open(true)
        .show(ui, |ui| {
            let Some(doc) = &app.doc else {
                ui.label(app.i18n.t("sidebar-empty", &[]));
                return;
            };
            let w = doc.width();
            let h = doc.height();
            let total_px = (w as u64) * (h as u64);
            let est_mem_mb = (total_px * 4) as f64 / (1024.0 * 1024.0);

            egui::Grid::new("props-grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(app.i18n.t("prop-dimensions", &[]));
                    ui.label(format!("{} × {} px", w, h));
                    ui.end_row();

                    ui.label(app.i18n.t("prop-megapixels", &[]));
                    ui.label(format!("{:.2} MP", total_px as f64 / 1_000_000.0));
                    ui.end_row();

                    ui.label(app.i18n.t("prop-aspect", &[]));
                    ui.label(aspect_ratio_string(w, h));
                    ui.end_row();

                    ui.label(app.i18n.t("prop-channels", &[]));
                    ui.label("RGBA 8-bit");
                    ui.end_row();

                    ui.label(app.i18n.t("prop-memory", &[]));
                    ui.label(format!("{:.2} MB", est_mem_mb));
                    ui.end_row();

                    if let Some(path) = &doc.path {
                        ui.label(app.i18n.t("prop-format", &[]));
                        let ext = path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_ascii_uppercase();
                        ui.label(if ext.is_empty() {
                            "—".to_string()
                        } else {
                            ext
                        });
                        ui.end_row();

                        if let Ok(meta) = std::fs::metadata(path) {
                            ui.label(app.i18n.t("prop-file-size", &[]));
                            ui.label(format_bytes(meta.len()));
                            ui.end_row();

                            if let Ok(modified) = meta.modified() {
                                ui.label(app.i18n.t("prop-modified", &[]));
                                ui.label(format_time(modified));
                                ui.end_row();
                            }
                        }
                    }
                });

            if let Some(path) = &doc.path {
                ui.add_space(4.0);
                ui.label(app.i18n.t("prop-path", &[]));
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(path.display().to_string())
                            .monospace()
                            .small(),
                    )
                    .wrap(),
                );
            }
        });
}

fn show_tool_section(ui: &mut egui::Ui, app: &mut YImageApp) {
    let title = match app.tool {
        ToolKind::None => app.i18n.t("inspector-no-tool", &[]),
        _ => app.i18n.t("inspector-tool", &[]),
    };
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| match app.tool {
            ToolKind::Draw => draw_panel(ui, app),
            ToolKind::Mosaic => mosaic_panel(ui, app),
            ToolKind::Text => text_panel(ui, app),
            ToolKind::Shape => shape_panel(ui, app),
            ToolKind::BackgroundRemove => bg_remove_panel(ui, app),
            ToolKind::ObjectRemove => obj_remove_panel(ui, app),
            ToolKind::Gif => gif_panel(ui, app),
            ToolKind::None => {
                ui.label(app.i18n.t("inspector-pick-tool", &[]));
            }
        });
}

fn draw_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
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
    // Eraser toggle
    ui.checkbox(
        &mut app.dialog.brush.eraser,
        app.i18n.t("brush-eraser", &[]),
    );
}

fn mosaic_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.label(app.i18n.t("tool-mosaic", &[]));
    ui.add(
        egui::Slider::new(&mut app.dialog.mosaic.block_size, 2..=128)
            .text(app.i18n.t("mosaic-block-size", &[])),
    );
    ui.label(app.i18n.t("mosaic-hint", &[]));
}

fn text_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.label(app.i18n.t("tool-text", &[]));
    ui.add(
        egui::TextEdit::multiline(&mut app.dialog.text.content)
            .hint_text(app.i18n.t("text-hint", &[]))
            .desired_rows(3),
    );
    ui.add(
        egui::Slider::new(&mut app.dialog.text.font_size, 8.0..=256.0)
            .text(app.i18n.t("text-size", &[])),
    );
    let mut color = egui::Color32::from_rgba_unmultiplied(
        app.dialog.text.color[0],
        app.dialog.text.color[1],
        app.dialog.text.color[2],
        app.dialog.text.color[3],
    );
    if egui::color_picker::color_edit_button_srgba(
        ui,
        &mut color,
        egui::color_picker::Alpha::Opaque,
    )
    .changed()
    {
        app.dialog.text.color = [color.r(), color.g(), color.b(), color.a()];
    }
    ui.label(app.i18n.t("text-click-hint", &[]));
}

fn shape_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    use crate::tools::draw::ShapeKind;
    ui.label(app.i18n.t("tool-shape", &[]));
    egui::ComboBox::from_label(app.i18n.t("shape-kind", &[]))
        .selected_text(shape_name(app.dialog.shape.kind, &app.i18n))
        .show_ui(ui, |ui| {
            for kind in [
                ShapeKind::Rect,
                ShapeKind::RectFilled,
                ShapeKind::Ellipse,
                ShapeKind::EllipseFilled,
                ShapeKind::Line,
                ShapeKind::Arrow,
            ] {
                ui.selectable_value(
                    &mut app.dialog.shape.kind,
                    kind,
                    shape_name(kind, &app.i18n),
                );
            }
        });
    ui.add(
        egui::Slider::new(&mut app.dialog.shape.stroke, 1.0..=32.0)
            .text(app.i18n.t("shape-stroke", &[])),
    );
    let mut color = egui::Color32::from_rgba_unmultiplied(
        app.dialog.shape.color[0],
        app.dialog.shape.color[1],
        app.dialog.shape.color[2],
        app.dialog.shape.color[3],
    );
    if egui::color_picker::color_edit_button_srgba(
        ui,
        &mut color,
        egui::color_picker::Alpha::Opaque,
    )
    .changed()
    {
        app.dialog.shape.color = [color.r(), color.g(), color.b(), color.a()];
    }
}

fn shape_name(kind: crate::tools::draw::ShapeKind, i18n: &crate::i18n::I18n) -> String {
    use crate::tools::draw::ShapeKind;
    match kind {
        ShapeKind::Rect => i18n.t("shape-rect", &[]),
        ShapeKind::RectFilled => i18n.t("shape-rect-filled", &[]),
        ShapeKind::Ellipse => i18n.t("shape-ellipse", &[]),
        ShapeKind::EllipseFilled => i18n.t("shape-ellipse-filled", &[]),
        ShapeKind::Line => i18n.t("shape-line", &[]),
        ShapeKind::Arrow => i18n.t("shape-arrow", &[]),
    }
}

fn bg_remove_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.label(app.i18n.t("tool-bg-remove", &[]));
    ui.label(app.i18n.t("bg-remove-hint", &[]));
    let status = crate::models::check(crate::models::ModelKind::BgRemove);
    show_model_status(ui, app, &status, crate::models::ModelKind::BgRemove);
    if status.ready
        && ui
            .add_enabled(
                app.doc.is_some(),
                egui::Button::new(app.i18n.t("action-run", &[])),
            )
            .clicked()
    {
        app.run_background_remove();
    }
}

fn obj_remove_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.label(app.i18n.t("tool-obj-remove", &[]));
    ui.label(app.i18n.t("obj-remove-hint", &[]));
    let status = crate::models::check(crate::models::ModelKind::ObjRemove);
    show_model_status(ui, app, &status, crate::models::ModelKind::ObjRemove);
    if status.ready {
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
            app.run_object_remove();
        }
    }
}

fn show_model_status(
    ui: &mut egui::Ui,
    app: &mut YImageApp,
    status: &crate::models::ModelStatus,
    kind: crate::models::ModelKind,
) {
    ui.add_space(4.0);
    if status.ready {
        ui.colored_label(
            super::theme::ACCENT,
            app.i18n
                .t("model-ready", &[("size", format_bytes(status.size))]),
        );
    } else {
        ui.colored_label(
            egui::Color32::from_rgb(0xE8, 0xA0, 0x40),
            app.i18n.t("model-missing", &[]),
        );
        ui.label(
            egui::RichText::new(status.path.display().to_string())
                .monospace()
                .small(),
        );
        if app.download_state(kind).in_progress {
            let p = app.download_state(kind).progress;
            ui.add(egui::ProgressBar::new(p).show_percentage());
        } else if ui
            .button(app.i18n.t("action-download-model", &[]))
            .clicked()
        {
            app.download_model(kind);
        }
    }
}

fn gif_panel(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.label(app.i18n.t("tool-gif", &[]));
    ui.label(app.i18n.t("gif-open-builder-hint", &[]));
    if ui.button(app.i18n.t("gif-open-builder", &[])).clicked() {
        app.dialog.gif_timeline_open = true;
    }
}

fn aspect_ratio_string(w: u32, h: u32) -> String {
    let gcd = {
        let mut a = w;
        let mut b = h;
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a.max(1)
    };
    format!(
        "{}:{}  ({:.3})",
        w / gcd,
        h / gcd,
        w as f32 / h.max(1) as f32
    )
}

fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

fn format_time(t: std::time::SystemTime) -> String {
    match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs() as i64;
            // Naive YYYY-MM-DD HH:MM — avoids a chrono dependency.
            let (y, mo, d, h, mi) = civil_from_unix(secs);
            format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
        }
        Err(_) => "—".into(),
    }
}

/// Convert a Unix timestamp to (year, month, day, hour, minute) in UTC.
/// Based on Howard Hinnant's civil_from_days algorithm.
fn civil_from_unix(secs: i64) -> (i32, u8, u8, u8, u8) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let hour = (rem / 3600) as u8;
    let minute = ((rem % 3600) / 60) as u8;

    let z = days + 719468;
    let era = if z >= 0 {
        z / 146097
    } else {
        (z - 146096) / 146097
    };
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m, d, hour, minute)
}
