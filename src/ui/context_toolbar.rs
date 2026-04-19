// Contextual toolbar that docks to the top of the canvas area.
//
// Replaces the always-visible 260-px right-hand Inspector panel. Appears only
// when a tool is selected, showing exactly the controls relevant to that tool
// in a single compact horizontal row — drag handles, sliders, colour swatches,
// Run buttons, etc. Hidden when the `None` (select) tool is active so the
// canvas gets the whole viewport.

use egui::{Color32, CornerRadius, RichText, Sense, Stroke, Vec2};

use crate::app::YImageApp;
use crate::tools::ToolKind;
use crate::ui::theme;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    // `None` tool = pure viewing mode → nothing to show.
    if app.tool == ToolKind::None {
        return;
    }

    egui::TopBottomPanel::top("context_toolbar")
        .show_separator_line(false)
        .frame(theme::toolbar_frame(app.settings.theme_dark))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                match app.tool {
                    ToolKind::Draw => draw_controls(ui, app),
                    ToolKind::Mosaic => mosaic_controls(ui, app),
                    ToolKind::Text => text_controls(ui, app),
                    ToolKind::Shape => shape_controls(ui, app),
                    ToolKind::BackgroundRemove => bg_remove_controls(ui, app),
                    ToolKind::ObjectRemove => obj_remove_controls(ui, app),
                    ToolKind::Gif => gif_controls(ui, app),
                    ToolKind::None => {}
                }

                // Progress indicator for long-running ops (AI, export, etc.).
                if let Some((label, v)) = &app.progress {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add(
                                egui::ProgressBar::new(*v)
                                    .desired_width(120.0)
                                    .show_percentage(),
                            );
                            ui.label(
                                RichText::new(label.as_str())
                                    .size(theme::FONT_CAPTION)
                                    .color(secondary_text(app)),
                            );
                        },
                    );
                }
            });
        });
}

// ── Per-tool rows ──────────────────────────────────────────────────

fn draw_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    labeled_slider(
        ui,
        &app.i18n.t("brush-size", &[]),
        &mut app.dialog.brush.radius,
        1.0..=128.0,
        100.0,
    );
    labeled_slider(
        ui,
        &app.i18n.t("brush-hardness", &[]),
        &mut app.dialog.brush.hardness,
        0.0..=1.0,
        80.0,
    );
    color_swatch(ui, &mut app.dialog.brush.color);
    ui.checkbox(
        &mut app.dialog.brush.eraser,
        app.i18n.t("brush-eraser", &[]),
    );
}

fn mosaic_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    labeled_slider_i(
        ui,
        &app.i18n.t("mosaic-block-size", &[]),
        &mut app.dialog.mosaic.block_size,
        2..=128,
        120.0,
    );
    hint(ui, &app.i18n.t("mosaic-hint", &[]), app.settings.theme_dark);
}

fn text_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.add(
        egui::TextEdit::singleline(&mut app.dialog.text.content)
            .hint_text(app.i18n.t("text-hint", &[]))
            .desired_width(200.0),
    );
    labeled_slider(
        ui,
        &app.i18n.t("text-size", &[]),
        &mut app.dialog.text.font_size,
        8.0..=256.0,
        100.0,
    );
    color_swatch(ui, &mut app.dialog.text.color);
    hint(
        ui,
        &app.i18n.t("text-click-hint", &[]),
        app.settings.theme_dark,
    );
}

fn shape_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    use crate::tools::draw::ShapeKind;
    egui::ComboBox::from_id_salt("shape_kind_combo")
        .selected_text(shape_name(app.dialog.shape.kind, &app.i18n))
        .width(140.0)
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
    labeled_slider(
        ui,
        &app.i18n.t("shape-stroke", &[]),
        &mut app.dialog.shape.stroke,
        1.0..=32.0,
        100.0,
    );
    color_swatch(ui, &mut app.dialog.shape.color);
}

fn bg_remove_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    let status = crate::models::check(crate::models::ModelKind::BgRemove);
    show_model_inline(ui, app, &status, crate::models::ModelKind::BgRemove);
    if status.ready
        && ui
            .add_enabled(
                app.has_doc(),
                egui::Button::new(format!("\u{25B6} {}", app.i18n.t("action-run", &[])))
                    .fill(theme::ACCENT)
                    .corner_radius(CornerRadius::same(8)),
            )
            .clicked()
    {
        app.run_background_remove();
    }
}

fn obj_remove_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    let status = crate::models::check(crate::models::ModelKind::ObjRemove);
    show_model_inline(ui, app, &status, crate::models::ModelKind::ObjRemove);
    if status.ready {
        if ui
            .button(app.i18n.t("action-clear-mask", &[]))
            .clicked()
        {
            app.dialog.obj_mask = None;
        }
        if ui
            .add_enabled(
                app.has_doc() && app.dialog.obj_mask.is_some(),
                egui::Button::new(format!("\u{25B6} {}", app.i18n.t("action-run", &[])))
                    .fill(theme::ACCENT)
                    .corner_radius(CornerRadius::same(8)),
            )
            .clicked()
        {
            app.run_object_remove();
        }
    }
    hint(
        ui,
        &app.i18n.t("obj-remove-hint", &[]),
        app.settings.theme_dark,
    );
}

fn gif_controls(ui: &mut egui::Ui, app: &mut YImageApp) {
    if ui
        .add(
            egui::Button::new(format!(
                "\u{1F39E}  {}",
                app.i18n.t("gif-open-builder", &[])
            ))
            .fill(theme::ACCENT)
            .corner_radius(CornerRadius::same(8)),
        )
        .clicked()
    {
        app.dialog.gif_timeline_open = true;
    }
    hint(
        ui,
        &app.i18n.t("gif-open-builder-hint", &[]),
        app.settings.theme_dark,
    );
}

// ── Helpers ────────────────────────────────────────────────────────

fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    width: f32,
) {
    ui.label(
        RichText::new(label)
            .size(theme::FONT_CAPTION)
            .color(Color32::from_gray(150)),
    );
    ui.spacing_mut().slider_width = width;
    ui.add(egui::Slider::new(value, range).show_value(true));
}

fn labeled_slider_i(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    width: f32,
) {
    ui.label(
        RichText::new(label)
            .size(theme::FONT_CAPTION)
            .color(Color32::from_gray(150)),
    );
    ui.spacing_mut().slider_width = width;
    ui.add(egui::Slider::new(value, range).show_value(true));
}

fn color_swatch(ui: &mut egui::Ui, color_rgba: &mut [u8; 4]) {
    // Chunky 26x26 swatch with rounded corners — easier to hit than egui's
    // default thin button.
    let mut c =
        Color32::from_rgba_unmultiplied(color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), c);
    painter.rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, Color32::from_black_alpha(80)),
        egui::StrokeKind::Inside,
    );
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(resp.id));
    }
    egui::Popup::from_toggle_button_response(&resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui: &mut egui::Ui| {
            if egui::color_picker::color_picker_color32(
                ui,
                &mut c,
                egui::color_picker::Alpha::Opaque,
            ) {
                *color_rgba = [c.r(), c.g(), c.b(), c.a()];
            }
        });
    // Keep the change even when the picker is closed via outside click.
    *color_rgba = [c.r(), c.g(), c.b(), c.a()];
}

fn hint(ui: &mut egui::Ui, text: &str, dark: bool) {
    let color = if dark {
        theme::TEXT_SECONDARY_DARK
    } else {
        theme::TEXT_SECONDARY_LIGHT
    };
    ui.label(RichText::new(text).size(theme::FONT_CAPTION).color(color));
}

fn secondary_text(app: &YImageApp) -> Color32 {
    if app.settings.theme_dark {
        theme::TEXT_SECONDARY_DARK
    } else {
        theme::TEXT_SECONDARY_LIGHT
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

fn show_model_inline(
    ui: &mut egui::Ui,
    app: &mut YImageApp,
    status: &crate::models::ModelStatus,
    kind: crate::models::ModelKind,
) {
    if status.ready {
        ui.colored_label(
            theme::SUCCESS,
            app.i18n
                .t("model-ready", &[("size", format_bytes(status.size))]),
        );
    } else {
        ui.colored_label(theme::WARNING, app.i18n.t("model-missing", &[]));
        if app.download_state(kind).in_progress {
            let p = app.download_state(kind).progress;
            ui.add(egui::ProgressBar::new(p).desired_width(120.0).show_percentage());
        } else if ui
            .button(app.i18n.t("action-download-model", &[]))
            .clicked()
        {
            app.download_model(kind);
        }
    }
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
