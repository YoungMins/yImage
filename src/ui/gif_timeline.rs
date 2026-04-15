// GIF builder — dedicated tool with a timeline UI.
//
// Rather than hiding GIF creation behind a modal dialog, this opens as a
// full-viewport workspace when the user picks Tool → "GIF Builder". Frames
// are laid out horizontally on a timeline; users can add, remove, and
// reorder them, then set per-timeline delay + loop count and export.

use std::path::PathBuf;

use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Vec2};

use crate::app::{BgMsg, YImageApp};
use crate::ui::thumbnails::THUMB_MAX_DIM;

#[derive(Default)]
pub struct GifTimelineState {
    pub frames: Vec<GifFrame>,
    pub delay_ms: u16,
    pub loop_infinite: bool,
    pub selected: Option<usize>,
    /// Open flag so this also works as a dockable workspace. Closing the
    /// workspace keeps the frames around so reopening is zero-click.
    pub open: bool,
}

#[derive(Clone)]
pub struct GifFrame {
    pub path: PathBuf,
    pub texture: Option<TextureHandle>,
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if !app.dialog.gif_timeline_open {
        return;
    }
    let mut open = app.dialog.gif_timeline_open;
    egui::Window::new(app.i18n.t("gif-builder-title", &[]))
        .open(&mut open)
        .default_size([820.0, 520.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            controls_bar(ui, app);
            ui.separator();
            preview_area(ctx, ui, app);
            ui.separator();
            timeline_strip(ctx, ui, app);
            ui.separator();
            export_bar(ui, app);
        });
    app.dialog.gif_timeline_open = open;
}

fn controls_bar(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.horizontal(|ui| {
        if ui.button(app.i18n.t("gif-add-frames", &[])).clicked() {
            if let Some(files) = rfd::FileDialog::new()
                .add_filter(
                    "images",
                    &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"],
                )
                .pick_files()
            {
                for p in files {
                    app.dialog.gif.frames.push(GifFrame {
                        path: p,
                        texture: None,
                    });
                }
            }
        }
        if ui
            .add_enabled(
                app.dialog.gif.selected.is_some(),
                egui::Button::new(app.i18n.t("gif-remove-frame", &[])),
            )
            .clicked()
        {
            if let Some(i) = app.dialog.gif.selected.take() {
                if i < app.dialog.gif.frames.len() {
                    app.dialog.gif.frames.remove(i);
                }
            }
        }
        if ui.button(app.i18n.t("gif-clear", &[])).clicked() {
            app.dialog.gif.frames.clear();
            app.dialog.gif.selected = None;
        }
        ui.separator();
        ui.label(format!(
            "{} {}",
            app.dialog.gif.frames.len(),
            app.i18n.t("gif-frames", &[])
        ));
    });
}

fn preview_area(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    let sel = app.dialog.gif.selected;
    let avail = ui.available_rect_before_wrap();
    let preview_h = (avail.height() * 0.55).max(180.0);
    ui.allocate_ui_with_layout(
        Vec2::new(avail.width(), preview_h),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            match sel.and_then(|i| app.dialog.gif.frames.get(i).cloned()) {
                Some(frame) => {
                    let tex = ensure_gif_texture(ctx, &frame.path);
                    match tex {
                        Some(t) => {
                            let max_w = ui.available_width();
                            let max_h = ui.available_height();
                            let size = t.size_vec2();
                            let scale = (max_w / size.x).min(max_h / size.y).clamp(0.05, 1.0);
                            let disp = size * scale;
                            ui.add(egui::Image::new((t.id(), disp)).fit_to_exact_size(disp));
                            // Write back texture to cached slot.
                            if let Some(i) = sel {
                                if let Some(f) = app.dialog.gif.frames.get_mut(i) {
                                    f.texture = Some(t);
                                }
                            }
                        }
                        None => {
                            ui.label("…");
                        }
                    }
                }
                None => {
                    ui.label(app.i18n.t("gif-no-selection", &[]));
                }
            }
        },
    );
}

fn timeline_strip(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    let mut new_sel = app.dialog.gif.selected;
    let mut move_request: Option<(usize, isize)> = None;

    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, frame) in app.dialog.gif.frames.iter_mut().enumerate() {
                    let is_sel = new_sel == Some(i);
                    let tex = match &frame.texture {
                        Some(t) => Some(t.clone()),
                        None => {
                            let t = ensure_gif_texture(ctx, &frame.path);
                            if let Some(t) = &t {
                                frame.texture = Some(t.clone());
                            }
                            t
                        }
                    };
                    let frame_ui = egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(3))
                        .corner_radius(egui::CornerRadius::same(6))
                        .stroke(if is_sel {
                            egui::Stroke::new(2.0, super::theme::ACCENT)
                        } else {
                            egui::Stroke::new(1.0, Color32::from_gray(80))
                        });
                    let r = frame_ui
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let size = Vec2::splat(96.0);
                                if let Some(t) = tex {
                                    ui.add(
                                        egui::Image::new((t.id(), size)).fit_to_exact_size(size),
                                    );
                                } else {
                                    let (rect, _) =
                                        ui.allocate_exact_size(size, egui::Sense::hover());
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::CornerRadius::same(4),
                                        Color32::from_gray(40),
                                    );
                                }
                                ui.label(format!("#{}", i + 1));
                                ui.horizontal(|ui| {
                                    if ui.small_button("◀").clicked() {
                                        move_request = Some((i, -1));
                                    }
                                    if ui.small_button("▶").clicked() {
                                        move_request = Some((i, 1));
                                    }
                                });
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    if r.clicked() {
                        new_sel = Some(i);
                    }
                }
            });
        });

    app.dialog.gif.selected = new_sel;

    if let Some((i, delta)) = move_request {
        let target = i as isize + delta;
        if target >= 0 && (target as usize) < app.dialog.gif.frames.len() {
            app.dialog.gif.frames.swap(i, target as usize);
            if app.dialog.gif.selected == Some(i) {
                app.dialog.gif.selected = Some(target as usize);
            }
        }
    }
}

fn export_bar(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.horizontal(|ui| {
        if app.dialog.gif.delay_ms == 0 {
            app.dialog.gif.delay_ms = 100;
        }
        ui.add(
            egui::Slider::new(&mut app.dialog.gif.delay_ms, 20..=2000)
                .text(app.i18n.t("gif-delay-ms", &[])),
        );
        ui.checkbox(
            &mut app.dialog.gif.loop_infinite,
            app.i18n.t("gif-loop-infinite", &[]),
        );
        if ui
            .add_enabled(
                !app.dialog.gif.frames.is_empty(),
                egui::Button::new(app.i18n.t("gif-export", &[])),
            )
            .clicked()
        {
            export(app);
        }
    });
}

fn export(app: &mut YImageApp) {
    let Some(out) = rfd::FileDialog::new()
        .set_file_name("animation.gif")
        .add_filter("gif", &["gif"])
        .save_file()
    else {
        return;
    };
    let inputs: Vec<PathBuf> = app
        .dialog
        .gif
        .frames
        .iter()
        .map(|f| f.path.clone())
        .collect();
    let delay = app.dialog.gif.delay_ms;
    let loop_count = if app.dialog.gif.loop_infinite { 0 } else { 1 };
    let tx = app.tx.clone();
    rayon::spawn(move || {
        let opts = crate::ops::gif::GifOptions {
            delay_ms: delay,
            loop_count,
            target_size: None,
        };
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

fn ensure_gif_texture(ctx: &egui::Context, path: &std::path::Path) -> Option<TextureHandle> {
    let img = crate::io::load::load_image(path).ok()?;
    let (tw, th) = {
        let max = THUMB_MAX_DIM;
        if img.width() <= max && img.height() <= max {
            (img.width(), img.height())
        } else if img.width() >= img.height() {
            (
                max,
                (max as f32 * img.height() as f32 / img.width() as f32) as u32,
            )
        } else {
            (
                (max as f32 * img.width() as f32 / img.height() as f32) as u32,
                max,
            )
        }
    };
    let small = crate::ops::resize::resize_rgba(
        &img,
        tw.max(1),
        th.max(1),
        crate::ops::resize::Filter::Bilinear,
    )
    .ok()?;
    let size = [small.width() as usize, small.height() as usize];
    let pixels: Vec<Color32> = small
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(ctx.load_texture(
        format!("gif:{}", path.display()),
        ColorImage {
            size,
            source_size: egui::vec2(size[0] as f32, size[1] as f32),
            pixels,
        },
        TextureOptions::LINEAR,
    ))
}
