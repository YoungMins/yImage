// Modal dialogs (shown as egui Windows) for Resize, Convert, Optimize, Save-as.
// Per-tool state also lives on DialogState to avoid stuffing it onto the main
// App struct. The GIF builder has graduated to its own timeline workspace
// (`ui::gif_timeline`) and no longer uses a modal dialog.

use std::path::PathBuf;

use image::GrayImage;

use crate::app::{BgMsg, YImageApp};
use crate::ops::resize::{aspect_fit, resize_rgba, Filter};
use crate::tools::{
    draw::{BrushState, ShapeState, TextState},
    mosaic::MosaicState,
};
use crate::ui::gif_timeline::GifTimelineState;
use crate::ui::theme;
use egui::{CornerRadius, RichText, Vec2};

/// Primary CTA button — accent-filled pill that stands out inside a dialog.
fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(13.0)
                .color(egui::Color32::WHITE),
        )
        .min_size(Vec2::new(110.0, 32.0))
        .fill(theme::ACCENT)
        .corner_radius(CornerRadius::same(8)),
    )
}

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

    // ICO export
    pub ico_open: bool,
    pub ico_sizes: [bool; 4],

    // Save as
    pub save_dialog_open: bool,

    // GIF timeline workspace
    pub gif: GifTimelineState,
    pub gif_timeline_open: bool,

    // Tool state
    pub brush: BrushState,
    pub mosaic: MosaicState,
    pub mosaic_start: Option<(f32, f32)>,
    pub text: TextState,
    pub shape: ShapeState,
    pub shape_start: Option<(f32, f32)>,
    pub crop_start: Option<(f32, f32)>,
    /// Confirmed crop rectangle in image coordinates (x, y, w, h).
    /// Set when the user finishes a drag; cleared on Apply / Cancel.
    pub crop_rect: Option<(u32, u32, u32, u32)>,
    pub obj_mask: Option<GrayImage>,
    pub obj_mask_tex: Option<egui::TextureHandle>,

    // Fixed-region capture rectangle (in screen coordinates).
    pub fixed_region: Option<(i32, i32, u32, u32)>,

    // Region-selection overlay shown after a Region / FixedRegion capture.
    // Holds the full-screen screenshot and the user's drag-rectangle state
    // until they confirm (crop+open) or cancel.
    #[cfg(all(windows, feature = "capture"))]
    pub region_crop: Option<crate::ui::capture_overlay::RegionCropState>,

    // Countdown before firing an ActiveWindow / AutoScroll capture so the
    // user has time to click on the target window.
    #[cfg(all(windows, feature = "capture"))]
    pub capture_countdown: Option<crate::ui::capture_overlay::CaptureCountdown>,

    // Hotkeys dialog
    pub hotkeys_open: bool,
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
    if app.dialog.ico_open {
        ico_dialog(ctx, app);
    }
    if app.dialog.save_dialog_open {
        save_as_dialog(app);
        app.dialog.save_dialog_open = false;
    }
    if app.dialog.hotkeys_open {
        hotkeys_dialog(ctx, app);
    }
}

fn resize_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    let Some(doc) = app.active_doc() else {
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
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.add_space(theme::SPACE_XS);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SPACE_SM;
                ui.label(RichText::new("W").size(theme::FONT_CAPTION));
                let r = ui.add(
                    egui::DragValue::new(&mut app.dialog.resize_w)
                        .range(1..=65535)
                        .speed(1.0),
                );
                if r.changed() && app.dialog.resize_keep_aspect {
                    let (_, h) = aspect_fit(src_w, src_h, app.dialog.resize_w, 0);
                    app.dialog.resize_h = h;
                }
                ui.label(if app.dialog.resize_keep_aspect { "\u{1F517}" } else { "\u{2715}" });
                ui.label(RichText::new("H").size(theme::FONT_CAPTION));
                let r = ui.add(
                    egui::DragValue::new(&mut app.dialog.resize_h)
                        .range(1..=65535)
                        .speed(1.0),
                );
                if r.changed() && app.dialog.resize_keep_aspect {
                    let (w, _) = aspect_fit(src_w, src_h, 0, app.dialog.resize_h);
                    app.dialog.resize_w = w;
                }
            });
            ui.add_space(theme::SPACE_SM);
            ui.checkbox(
                &mut app.dialog.resize_keep_aspect,
                app.i18n.t("resize-lock-aspect", &[]),
            );
            ui.add_space(theme::SPACE_SM);
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
            ui.add_space(theme::SPACE_MD);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, &app.i18n.t("action-apply", &[])).clicked() {
                    apply = true;
                }
            });
        });
    app.dialog.resize_open = open;

    if apply {
        // Extract dialog values before borrowing the tab mutably so the
        // compiler sees disjoint borrows of app.dialog and app.tabs.
        let target_w = app.dialog.resize_w;
        let target_h = app.dialog.resize_h;
        let filter = app.dialog.resize_filter.to_filter();
        let idx = app.active_tab;
        let result = app
            .tabs
            .get(idx)
            .map(|tab| resize_rgba(&tab.doc.image, target_w, target_h, filter));
        match result {
            Some(Ok(new_img)) => {
                if let Some(tab) = app.tabs.get_mut(idx) {
                    tab.doc.replace(new_img);
                    tab.texture_dirty = true;
                }
                app.dialog.resize_open = false;
            }
            Some(Err(e)) => {
                let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
            }
            None => {}
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
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.add_space(theme::SPACE_XS);
            ui.label(
                RichText::new(app.i18n.t("convert-target", &[]))
                    .size(theme::FONT_CAPTION),
            );
            ui.add_space(theme::SPACE_XS);
            // Format tiles laid out in a grid — click a tile to pick the
            // target format. Faster + more visual than a dropdown.
            let formats = ["png", "jpg", "webp", "bmp", "tiff", "gif", "avif", "ico"];
            egui::Grid::new("format_tiles")
                .num_columns(4)
                .spacing([theme::SPACE_SM, theme::SPACE_SM])
                .show(ui, |ui| {
                    for (i, ext) in formats.iter().enumerate() {
                        let selected = app.dialog.convert_target == *ext;
                        let fill = if selected {
                            theme::ACCENT
                        } else {
                            ui.visuals().widgets.inactive.weak_bg_fill
                        };
                        let text_color = if selected {
                            egui::Color32::WHITE
                        } else {
                            ui.visuals().text_color()
                        };
                        let r = ui.add(
                            egui::Button::new(
                                RichText::new(ext.to_uppercase())
                                    .size(13.0)
                                    .color(text_color),
                            )
                            .min_size(Vec2::new(66.0, 44.0))
                            .fill(fill)
                            .corner_radius(CornerRadius::same(8)),
                        );
                        if r.clicked() {
                            app.dialog.convert_target = ext.to_string();
                        }
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(theme::SPACE_MD);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, &app.i18n.t("action-save-as", &[])).clicked() {
                    pick_and_save = true;
                }
            });
        });
    app.dialog.convert_open = open;

    if pick_and_save {
        let Some(doc) = app.active_doc() else { return };
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
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.add_space(theme::SPACE_XS);
            ui.spacing_mut().slider_width = 200.0;
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
            ui.add_space(theme::SPACE_MD);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, &app.i18n.t("action-run", &[])).clicked() {
                    run = true;
                }
            });
        });
    app.dialog.optimize_open = open;

    if run {
        let Some(doc) = app.active_doc() else { return };
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
            rayon::spawn(
                move || match crate::io::optimize::optimize_to(&image, &out, &opts) {
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
                },
            );
        }
        app.dialog.optimize_open = false;
    }
}

/// Standard ICO resolutions. Modern Windows uses 256 for large-icon views; the
/// older 16/32/48 set is still required for legacy surfaces like tray icons,
/// taskbar, and file-explorer small-icon mode.
const ICO_SIZES: [u32; 4] = [16, 32, 48, 256];

fn ico_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    // If nothing is selected yet (fresh struct default), pre-check all sizes.
    if app.dialog.ico_sizes.iter().all(|&b| !b) {
        app.dialog.ico_sizes = [true; 4];
    }
    let mut open = app.dialog.ico_open;
    let mut export = false;
    egui::Window::new(app.i18n.t("action-export-ico", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.add_space(theme::SPACE_XS);
            ui.label(
                RichText::new(app.i18n.t("ico-sizes-hint", &[]))
                    .size(theme::FONT_CAPTION),
            );
            ui.add_space(theme::SPACE_SM);
            for (i, size) in ICO_SIZES.iter().enumerate() {
                ui.checkbox(
                    &mut app.dialog.ico_sizes[i],
                    format!("{size} × {size}"),
                );
            }
            ui.add_space(theme::SPACE_MD);
            let any_selected = app.dialog.ico_sizes.iter().any(|&b| b);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        any_selected,
                        egui::Button::new(
                            RichText::new(app.i18n.t("action-export", &[]))
                                .size(13.0)
                                .color(egui::Color32::WHITE),
                        )
                        .min_size(Vec2::new(110.0, 32.0))
                        .fill(theme::ACCENT)
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    export = true;
                }
            });
        });
    app.dialog.ico_open = open;

    if export {
        let Some(doc) = app.active_doc() else { return };
        let default_name = doc
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("icon")
            .to_string();
        let sizes: Vec<u32> = ICO_SIZES
            .iter()
            .zip(app.dialog.ico_sizes.iter())
            .filter_map(|(s, on)| if *on { Some(*s) } else { None })
            .collect();
        if let Some(out) = rfd::FileDialog::new()
            .set_file_name(format!("{default_name}.ico"))
            .add_filter("Windows Icon", &["ico"])
            .save_file()
        {
            let image = doc.image.clone();
            let tx = app.tx.clone();
            rayon::spawn(move || {
                if let Err(e) = crate::io::save::save_as_ico(&image, &out, &sizes) {
                    let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                } else {
                    let _ = tx.send(BgMsg::ImageSaved(out));
                }
            });
        }
        app.dialog.ico_open = false;
    }
}

fn save_as_dialog(app: &mut YImageApp) {
    let Some(doc) = app.active_doc() else { return };
    let default_name = doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or("image.png")
        .to_string();
    if let Some(out) = rfd::FileDialog::new()
        .set_file_name(default_name)
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
}

#[cfg(all(windows, feature = "capture"))]
fn hotkeys_dialog(ctx: &egui::Context, app: &mut YImageApp) {
    use crate::hotkeys::HotkeyAction;
    let mut open = app.dialog.hotkeys_open;
    let mut apply = false;
    egui::Window::new(app.i18n.t("hotkeys-title", &[]))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(app.i18n.t("hotkeys-hint", &[]));
            ui.separator();

            // Show current conflicts so the warning is live as the user edits.
            let conflicts = crate::hotkeys::HotkeyRegistry::detect_conflicts(&app.settings.hotkeys);

            for action in HotkeyAction::all() {
                let label = action_label(action, &app.i18n);
                let key = action.as_key().to_string();
                ui.horizontal(|ui| {
                    ui.label(label);
                    let entry = app.settings.hotkeys.entry(action).or_default();
                    let edit = egui::TextEdit::singleline(entry)
                        .hint_text("Ctrl+Shift+KeyA")
                        .desired_width(180.0);
                    ui.add(edit);
                    if let Some(other) = conflicts.get(&action) {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xE8, 0x80, 0x40),
                            format!("⚠ {}", action_label(*other, &app.i18n)),
                        );
                    }
                    if let Some(err) = app.hotkeys.as_ref().and_then(|r| r.errors.get(&action)) {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xE8, 0x60, 0x40),
                            format!("⚠ {err}"),
                        );
                    }
                    let _ = key;
                });
            }

            ui.separator();
            if ui.button(app.i18n.t("action-apply", &[])).clicked() {
                apply = true;
            }
        });
    app.dialog.hotkeys_open = open;
    if apply {
        app.apply_hotkeys();
    }
}

#[cfg(not(all(windows, feature = "capture")))]
fn hotkeys_dialog(_ctx: &egui::Context, _app: &mut YImageApp) {}

#[cfg(all(windows, feature = "capture"))]
fn action_label(action: crate::hotkeys::HotkeyAction, i18n: &crate::i18n::I18n) -> String {
    use crate::hotkeys::HotkeyAction;
    match action {
        HotkeyAction::CaptureFullscreen => i18n.t("cap-fullscreen", &[]),
        HotkeyAction::CaptureActiveWindow => i18n.t("cap-window", &[]),
        HotkeyAction::CaptureRegion => i18n.t("cap-region", &[]),
        HotkeyAction::CaptureFixedRegion => i18n.t("cap-fixed", &[]),
        HotkeyAction::CaptureAutoScroll => i18n.t("cap-scroll", &[]),
    }
}
