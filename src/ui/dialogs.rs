// Modal dialogs (shown as egui Windows) for Resize, Convert, Optimize, Save-as
// and GIF builder. Per-tool state also lives on DialogState to avoid stuffing
// it onto the main App struct.

use std::path::PathBuf;

use image::GrayImage;

use crate::app::{BgMsg, YImageApp};
use crate::ops::resize::{aspect_fit, resize_rgba, Filter};
use crate::tools::{draw::BrushState, mosaic::MosaicState};

#[derive(Default)]
pub struct DialogState {
    // Resize
    pub resize_open: bool,
    pub resize_w: u32,
    pub resize_h: u32,
    pub resize_keep_aspect: bool,
    pub resize_filter: FilterSel,

    // Convert
    pub convert_open: bool,
    pub convert_target: String,

    // Optimize
    pub optimize_open: bool,

    // Save as
    pub save_dialog_open: bool,

    // GIF
    pub gif_open: bool,
    pub gif_inputs: Vec<PathBuf>,
    pub gif_delay_ms: u16,

    // Tool state
    pub brush: BrushState,
    pub mosaic: MosaicState,
    pub mosaic_start: Option<(f32, f32)>,
    pub obj_mask: Option<GrayImage>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterSel {
    Nearest,
    Bilinear,
    #[default]
    Lanczos3,
}

impl FilterSel {
    fn to_filter(self) -> Filter {
        match self {
            FilterSel::Nearest => Filter::Nearest,
            FilterSel::Bilinear => Filter::Bilinear,
            FilterSel::Lanczos3 => Filter::Lanczos3,
        }
    }
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if app.dialog.resize_open {
        resize_dialog(ctx, app);
    }
    if app.dialog.convert_open {
        convert_dialog(ctx, app);
    }
    if app.dialog.optimize_open {
        optimize_dialog(ctx, app);
    }
    if app.dialog.save_dialog_open {
        save_as_dialog(app);
        app.dialog.save_dialog_open = false;
    }
    if app.dialog.gif_open {
        gif_dialog(ctx, app);
    }
}

fn resize_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    let Some(doc) = app.doc.as_ref() else {
        app.dialog.resize_open = false;
        return;
    };
    let (src_w, src_h) = (doc.width(), doc.height());
    if app.dialog.resize_w == 0 {
        app.dialog.resize_w = src_w;
    }
    if app.dialog.resize_h == 0 {
        app.dialog.resize_h = src_h;
    }

    let mut open = app.dialog.resize_open;
    let mut apply = false;
    egui::Window::new(app.i18n.t("action-resize", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("W");
                let r = ui.add(egui::DragValue::new(&mut app.dialog.resize_w).range(1..=65535));
                if r.changed() && app.dialog.resize_keep_aspect {
                    let (_, h) = aspect_fit(src_w, src_h, app.dialog.resize_w, 0);
                    app.dialog.resize_h = h;
                }
                ui.label("H");
                let r = ui.add(egui::DragValue::new(&mut app.dialog.resize_h).range(1..=65535));
                if r.changed() && app.dialog.resize_keep_aspect {
                    let (w, _) = aspect_fit(src_w, src_h, 0, app.dialog.resize_h);
                    app.dialog.resize_w = w;
                }
            });
            ui.checkbox(
                &mut app.dialog.resize_keep_aspect,
                app.i18n.t("resize-lock-aspect", &[]),
            );
            egui::ComboBox::from_label(app.i18n.t("resize-filter", &[]))
                .selected_text(format!("{:?}", app.dialog.resize_filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.dialog.resize_filter,
                        FilterSel::Nearest,
                        "Nearest",
                    );
                    ui.selectable_value(
                        &mut app.dialog.resize_filter,
                        FilterSel::Bilinear,
                        "Bilinear",
                    );
                    ui.selectable_value(
                        &mut app.dialog.resize_filter,
                        FilterSel::Lanczos3,
                        "Lanczos3",
                    );
                });
            if ui.button(app.i18n.t("action-apply", &[])).clicked() {
                apply = true;
            }
        });
    app.dialog.resize_open = open;

    if apply {
        if let Some(doc) = app.doc.as_mut() {
            match resize_rgba(
                &doc.image,
                app.dialog.resize_w,
                app.dialog.resize_h,
                app.dialog.resize_filter.to_filter(),
            ) {
                Ok(new_img) => {
                    doc.replace(new_img);
                    app.texture_dirty = true;
                    app.dialog.resize_open = false;
                }
                Err(e) => {
                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
        }
    }
}

fn convert_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    let mut open = app.dialog.convert_open;
    let mut pick_and_save = false;
    if app.dialog.convert_target.is_empty() {
        app.dialog.convert_target = "png".to_string();
    }
    egui::Window::new(app.i18n.t("action-convert", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            egui::ComboBox::from_label(app.i18n.t("convert-target", &[]))
                .selected_text(&app.dialog.convert_target)
                .show_ui(ui, |ui| {
                    for ext in ["png", "jpg", "webp", "bmp", "tiff", "gif", "avif"] {
                        ui.selectable_value(
                            &mut app.dialog.convert_target,
                            ext.to_string(),
                            ext,
                        );
                    }
                });
            if ui.button(app.i18n.t("action-save-as", &[])).clicked() {
                pick_and_save = true;
            }
        });
    app.dialog.convert_open = open;

    if pick_and_save {
        let Some(doc) = app.doc.as_ref() else { return };
        let default_name = doc
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();
        if let Some(out) = rfd::FileDialog::new()
            .set_file_name(format!("{default_name}.{}", app.dialog.convert_target))
            .add_filter("image", &[app.dialog.convert_target.as_str()])
            .save_file()
        {
            let image = doc.image.clone();
            let tx = app.tx.clone();
            rayon::spawn(move || {
                if let Err(e) = crate::io::save::save_image(&image, &out) {
                    let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                } else {
                    let _ = tx.send(BgMsg::ImageSaved(out));
                }
            });
        }
        app.dialog.convert_open = false;
    }
}

fn optimize_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    let mut open = app.dialog.optimize_open;
    let mut run = false;
    egui::Window::new(app.i18n.t("action-optimize", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add(
                egui::Slider::new(&mut app.settings.jpeg_quality, 40..=95)
                    .text(app.i18n.t("optimize-jpeg-quality", &[])),
            );
            ui.add(
                egui::Slider::new(&mut app.settings.png_level, 0..=6)
                    .text(app.i18n.t("optimize-png-level", &[])),
            );
            ui.add(
                egui::Slider::new(&mut app.settings.webp_quality, 40..=95)
                    .text(app.i18n.t("optimize-webp-quality", &[])),
            );
            if ui.button(app.i18n.t("action-run", &[])).clicked() {
                run = true;
            }
        });
    app.dialog.optimize_open = open;

    if run {
        let Some(doc) = app.doc.as_ref() else { return };
        let default_path = doc
            .path
            .clone()
            .map(|p| crate::io::optimize::default_out_path(&p))
            .unwrap_or_else(|| PathBuf::from("optimized.png"));
        if let Some(out) = rfd::FileDialog::new()
            .set_file_name(
                default_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("optimized.png"),
            )
            .save_file()
        {
            let image = doc.image.clone();
            let opts = crate::io::optimize::OptimizeOptions {
                jpeg_quality: app.settings.jpeg_quality,
                png_level: app.settings.png_level,
                webp_quality: app.settings.webp_quality,
            };
            let tx = app.tx.clone();
            rayon::spawn(move || {
                match crate::io::optimize::optimize_to(&image, &out, &opts) {
                    Ok(size) => {
                        let _ = tx.send(BgMsg::Info(format!(
                            "optimized -> {} ({} bytes)",
                            out.display(),
                            size
                        )));
                    }
                    Err(e) => {
                        let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                    }
                }
            });
        }
        app.dialog.optimize_open = false;
    }
}

fn save_as_dialog(app: &mut YImageApp) {
    let Some(doc) = app.doc.as_ref() else { return };
    let default_name = doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or("image.png")
        .to_string();
    if let Some(out) = rfd::FileDialog::new().set_file_name(default_name).save_file() {
        let image = doc.image.clone();
        let tx = app.tx.clone();
        rayon::spawn(move || {
            if let Err(e) = crate::io::save::save_image(&image, &out) {
                let _ = tx.send(BgMsg::Error(format!("{e:#}")));
            } else {
                let _ = tx.send(BgMsg::ImageSaved(out));
            }
        });
    }
}

fn gif_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    let mut open = app.dialog.gif_open;
    let mut build = false;
    if app.dialog.gif_delay_ms == 0 {
        app.dialog.gif_delay_ms = 100;
    }
    egui::Window::new(app.i18n.t("action-gif", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if ui.button(app.i18n.t("gif-pick-frames", &[])).clicked() {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("images", &["png", "jpg", "jpeg", "webp", "bmp"])
                    .pick_files()
                {
                    app.dialog.gif_inputs = files;
                }
            }
            ui.label(format!("{} frames", app.dialog.gif_inputs.len()));
            ui.add(
                egui::Slider::new(&mut app.dialog.gif_delay_ms, 20..=1000)
                    .text(app.i18n.t("gif-delay-ms", &[])),
            );
            if ui
                .add_enabled(
                    !app.dialog.gif_inputs.is_empty(),
                    egui::Button::new(app.i18n.t("action-build", &[])),
                )
                .clicked()
            {
                build = true;
            }
        });
    app.dialog.gif_open = open;

    if build {
        if let Some(out) = rfd::FileDialog::new()
            .set_file_name("output.gif")
            .add_filter("gif", &["gif"])
            .save_file()
        {
            let inputs = app.dialog.gif_inputs.clone();
            let opts = crate::ops::gif::GifOptions {
                delay_ms: app.dialog.gif_delay_ms,
                ..Default::default()
            };
            let tx = app.tx.clone();
            rayon::spawn(move || {
                match crate::ops::gif::build_gif_from_paths(&inputs, &out, &opts) {
                    Ok(()) => {
                        let _ = tx.send(BgMsg::Info(format!("gif saved: {}", out.display())));
                    }
                    Err(e) => {
                        let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                    }
                }
            });
        }
        app.dialog.gif_open = false;
    }
}
