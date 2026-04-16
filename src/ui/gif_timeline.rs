// GIF builder — two-column layout with drag-to-reorder timeline.
//
// Left column:  1) Add frames, 2) Timeline with drag reorder, 3) Playback settings
// Right column: 4) Preview & export — fills available height for a larger preview.
// The window is unconstrained so it can be dragged beyond the app bounds.

use std::path::PathBuf;
use std::time::Instant;

use egui::{Color32, ColorImage, Rect, RichText, Sense, TextureHandle, TextureOptions, Vec2};

use crate::app::{BgMsg, YImageApp};
use crate::ui::thumbnails::THUMB_MAX_DIM;

/// Maximum pixel dimension for the high-resolution preview texture.
const PREVIEW_MAX_DIM: u32 = 512;

pub struct GifTimelineState {
    pub frames: Vec<GifFrame>,
    pub delay_ms: u16,
    pub loop_infinite: bool,
    pub selected: Option<usize>,
    pub playing: bool,
    pub play_started: Option<Instant>,
    pub open: bool,
    /// Index of the frame currently being dragged (for reorder).
    pub drag_from: Option<usize>,
    /// Cached high-resolution preview texture for step 4 (path + handle).
    pub preview_tex: Option<(PathBuf, TextureHandle)>,
}

impl Default for GifTimelineState {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            delay_ms: 100,
            loop_infinite: true,
            selected: None,
            playing: false,
            play_started: None,
            open: false,
            drag_from: None,
            preview_tex: None,
        }
    }
}

#[derive(Clone)]
pub struct GifFrame {
    pub path: PathBuf,
    pub texture: Option<TextureHandle>,
}

/// Entry point: shows the GIF builder as a real OS-level window using egui's
/// multi-viewport support. Multi-viewport works with both the wgpu and glow
/// backends in eframe 0.27+, so the window is freely draggable anywhere on
/// the desktop (not confined to the main yImage window).
pub fn show_viewport(ctx: &egui::Context, app: &mut YImageApp) {
    if !app.dialog.gif_timeline_open {
        return;
    }

    if app.dialog.gif.playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    let title = format!("\u{1F39E}  {}", app.i18n.t("gif-builder-title", &[]));
    let viewport_id = egui::ViewportId::from_hash_of("yimage-gif-builder");
    let viewport_builder = egui::ViewportBuilder::default()
        .with_title(&title)
        .with_inner_size([1000.0, 620.0])
        .with_min_inner_size([720.0, 520.0]);

    ctx.show_viewport_immediate(viewport_id, viewport_builder, |vp_ctx, _class| {
        // Close button / window-close event from the OS.
        if vp_ctx.input(|i| i.viewport().close_requested()) {
            app.dialog.gif_timeline_open = false;
            return;
        }
        egui::CentralPanel::default().show(vp_ctx, |ui| {
            show_content(vp_ctx, ui, app);
        });
    });
}

/// Render the GIF builder content into any Ui (viewport or embedded).
pub fn show_content(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    if app.dialog.gif.playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    if app.dialog.gif.frames.is_empty() {
        empty_state(ui, app);
    } else {
        egui::SidePanel::right("gif-preview-panel")
            .default_width(320.0)
            .min_width(240.0)
            .max_width(500.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                step4_preview_export(ctx, ui, app);
            });

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                step1_add_frames(ui, app);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                step2_timeline(ctx, ui, app);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                step3_playback(ui, app);
                ui.add_space(12.0);
            });
    }
}

fn empty_state(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("\u{1F39E}")
                .size(64.0)
                .color(Color32::from_gray(140)),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(app.i18n.t("gif-builder-title", &[]))
                .size(22.0)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(app.i18n.t("gif-empty-state", &[]))
                .size(13.0)
                .color(Color32::from_gray(170)),
        );
        ui.add_space(20.0);
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("  {}  ", app.i18n.t("gif-add-frames", &[])))
                        .size(14.0)
                        .color(Color32::WHITE),
                )
                .min_size(Vec2::new(180.0, 40.0))
                .fill(super::theme::ACCENT),
            )
            .clicked()
        {
            pick_frames(app);
        }
        ui.add_space(40.0);
    });
}

fn step1_add_frames(ui: &mut egui::Ui, app: &mut YImageApp) {
    section_header(ui, &app.i18n.t("gif-step1", &[]));
    ui.weak(app.i18n.t("gif-step1-hint", &[]));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(format!(
                    "\u{2795}  {}",
                    app.i18n.t("gif-add-frames", &[])
                ))
                .min_size(Vec2::new(140.0, 28.0)),
            )
            .clicked()
        {
            pick_frames(app);
        }
        if ui
            .add_enabled(
                !app.dialog.gif.frames.is_empty(),
                egui::Button::new(format!("\u{1F5D1}  {}", app.i18n.t("gif-clear", &[])))
                    .min_size(Vec2::new(120.0, 28.0)),
            )
            .clicked()
        {
            app.dialog.gif.frames.clear();
            app.dialog.gif.selected = None;
            app.dialog.gif.playing = false;
            app.dialog.gif.play_started = None;
            app.dialog.gif.drag_from = None;
            app.dialog.gif.preview_tex = None;
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{}  {}",
                app.dialog.gif.frames.len(),
                app.i18n.t("gif-frames", &[])
            ))
            .color(Color32::from_gray(180)),
        );
    });
}

fn step2_timeline(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    section_header(ui, &app.i18n.t("gif-step2", &[]));
    ui.weak(app.i18n.t("gif-step2-hint", &[]));
    ui.add_space(6.0);

    let mut new_sel = app.dialog.gif.selected;
    let mut move_request: Option<(usize, isize)> = None;
    let mut remove_request: Option<usize> = None;
    let mut card_rects: Vec<Rect> = Vec::new();
    let mut drag_started_on: Option<usize> = None;

    egui::ScrollArea::horizontal()
        .id_salt("gif-timeline-scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let n = app.dialog.gif.frames.len();
                for i in 0..n {
                    let is_sel = new_sel == Some(i);
                    let is_dragging = app.dialog.gif.drag_from == Some(i);

                    let path = app.dialog.gif.frames[i].path.clone();
                    let cached = app.dialog.gif.frames[i].texture.clone();
                    let tex = cached.or_else(|| {
                        let t = ensure_thumb_texture(ctx, &path);
                        if let Some(t) = &t {
                            if let Some(f) = app.dialog.gif.frames.get_mut(i) {
                                f.texture = Some(t.clone());
                            }
                        }
                        t
                    });

                    const THUMB: f32 = 80.0;
                    const CARD_W: f32 = THUMB + 16.0;

                    let card = egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(4))
                        .corner_radius(egui::CornerRadius::same(6))
                        .fill(if is_dragging {
                            super::theme::ACCENT.linear_multiply(0.12)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(if is_sel {
                            egui::Stroke::new(2.0, super::theme::ACCENT)
                        } else if is_dragging {
                            egui::Stroke::new(2.0, super::theme::ACCENT.linear_multiply(0.6))
                        } else {
                            egui::Stroke::new(1.0, Color32::from_gray(80))
                        });

                    let resp = card
                        .show(ui, |ui| {
                            ui.set_width(CARD_W);
                            ui.vertical_centered(|ui| {
                                let size = Vec2::splat(THUMB);
                                if let Some(t) = tex {
                                    ui.add(
                                        egui::Image::new((t.id(), size))
                                            .fit_to_exact_size(size),
                                    );
                                } else {
                                    let (rect, _) =
                                        ui.allocate_exact_size(size, Sense::hover());
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::CornerRadius::same(4),
                                        Color32::from_gray(40),
                                    );
                                }
                                ui.label(
                                    RichText::new(format!("#{}", i + 1))
                                        .size(11.0)
                                        .color(if is_sel {
                                            super::theme::ACCENT
                                        } else {
                                            Color32::from_gray(180)
                                        }),
                                );
                                // Center the arrow/delete buttons
                                ui.horizontal(|ui| {
                                    let spacing = ui.spacing().item_spacing.x;
                                    let btn_approx_w = 22.0;
                                    let total_btns = 3.0 * btn_approx_w + 2.0 * spacing;
                                    let left_pad = ((CARD_W - total_btns) / 2.0).max(0.0);
                                    ui.add_space(left_pad);
                                    if ui
                                        .small_button("\u{25C0}")
                                        .on_hover_text(app.i18n.t("gif-move-left", &[]))
                                        .clicked()
                                    {
                                        move_request = Some((i, -1));
                                    }
                                    if ui
                                        .small_button("\u{1F5D1}")
                                        .on_hover_text(app.i18n.t("gif-remove-frame", &[]))
                                        .clicked()
                                    {
                                        remove_request = Some(i);
                                    }
                                    if ui
                                        .small_button("\u{25B6}")
                                        .on_hover_text(app.i18n.t("gif-move-right", &[]))
                                        .clicked()
                                    {
                                        move_request = Some((i, 1));
                                    }
                                });
                            });
                        })
                        .response
                        .interact(Sense::click_and_drag());

                    card_rects.push(resp.rect);

                    if resp.drag_started() {
                        drag_started_on = Some(i);
                    }
                    if resp.clicked() {
                        new_sel = Some(i);
                        app.dialog.gif.playing = false;
                        app.dialog.gif.play_started = None;
                    }
                }
            });
        });

    // Drag start
    if let Some(i) = drag_started_on {
        app.dialog.gif.drag_from = Some(i);
    }

    // Drag swap: when pointer hovers over a different card, swap frames
    if let Some(from) = app.dialog.gif.drag_from {
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            for (i, rect) in card_rects.iter().enumerate() {
                if i != from && rect.contains(pos) {
                    app.dialog.gif.frames.swap(from, i);
                    if app.dialog.gif.selected == Some(from) {
                        app.dialog.gif.selected = Some(i);
                    } else if app.dialog.gif.selected == Some(i) {
                        app.dialog.gif.selected = Some(from);
                    }
                    app.dialog.gif.preview_tex = None; // reset preview cache on reorder
                    app.dialog.gif.drag_from = Some(i);
                    break;
                }
            }
        }
    }

    // Drag end
    if app.dialog.gif.drag_from.is_some() && !ctx.input(|i| i.pointer.any_down()) {
        app.dialog.gif.drag_from = None;
    }

    app.dialog.gif.selected = new_sel;

    if let Some(i) = remove_request {
        if i < app.dialog.gif.frames.len() {
            app.dialog.gif.frames.remove(i);
            app.dialog.gif.preview_tex = None;
            if app.dialog.gif.selected == Some(i) {
                app.dialog.gif.selected = None;
            } else if let Some(s) = app.dialog.gif.selected {
                if s > i {
                    app.dialog.gif.selected = Some(s - 1);
                }
            }
        }
    }
    if let Some((i, delta)) = move_request {
        let target = i as isize + delta;
        if target >= 0 && (target as usize) < app.dialog.gif.frames.len() {
            app.dialog.gif.frames.swap(i, target as usize);
            app.dialog.gif.preview_tex = None;
            if app.dialog.gif.selected == Some(i) {
                app.dialog.gif.selected = Some(target as usize);
            } else if app.dialog.gif.selected == Some(target as usize) {
                app.dialog.gif.selected = Some(i);
            }
        }
    }

    ui.add_space(4.0);
    let total_ms = (app.dialog.gif.frames.len() as u32) * (app.dialog.gif.delay_ms as u32);
    let sel_idx = app.dialog.gif.selected.map(|i| i + 1).unwrap_or(0);
    ui.horizontal(|ui| {
        ui.weak(app.i18n.t(
            "gif-frame-index",
            &[
                ("i", sel_idx.to_string()),
                ("n", app.dialog.gif.frames.len().to_string()),
            ],
        ));
        ui.separator();
        ui.weak(app.i18n.t("gif-duration", &[("ms", total_ms.to_string())]));
    });
}

fn step3_playback(ui: &mut egui::Ui, app: &mut YImageApp) {
    section_header(ui, &app.i18n.t("gif-step3", &[]));
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
}

fn step4_preview_export(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    section_header(ui, &app.i18n.t("gif-step4", &[]));

    let frame_idx = current_preview_frame(&app.dialog.gif);

    // Preview fills available height minus controls at bottom.
    let avail = ui.available_rect_before_wrap();
    let controls_h = 96.0;
    let preview_h = (avail.height() - controls_h).max(120.0);

    ui.allocate_ui_with_layout(
        Vec2::new(avail.width(), preview_h),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            if let Some(i) = frame_idx {
                let path = app.dialog.gif.frames[i].path.clone();
                let tex = ensure_preview_texture(ctx, &mut app.dialog.gif, &path);
                match tex {
                    Some(t) => {
                        let max_w = ui.available_width();
                        let max_h = (ui.available_height() - 8.0).max(1.0);
                        let size = t.size_vec2();
                        let scale = (max_w / size.x).min(max_h / size.y).clamp(0.05, 4.0);
                        let disp = size * scale;
                        ui.add(egui::Image::new((t.id(), disp)).fit_to_exact_size(disp));
                    }
                    None => {
                        ui.add_space(40.0);
                        ui.weak("\u{2026}");
                    }
                }
            } else {
                ui.add_space(40.0);
                ui.weak(app.i18n.t("gif-preview-placeholder", &[]));
            }
        },
    );

    ui.add_space(6.0);

    // Play / Stop buttons — centered.
    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            let play_label = if app.dialog.gif.playing {
                format!("\u{23F8}  {}", app.i18n.t("gif-pause", &[]))
            } else {
                format!("\u{25B6}  {}", app.i18n.t("gif-play", &[]))
            };
            if ui
                .add_enabled(
                    app.dialog.gif.frames.len() >= 2,
                    egui::Button::new(play_label).min_size(Vec2::new(100.0, 28.0)),
                )
                .clicked()
            {
                if app.dialog.gif.playing {
                    app.dialog.gif.playing = false;
                    app.dialog.gif.play_started = None;
                } else {
                    app.dialog.gif.playing = true;
                    app.dialog.gif.play_started = Some(Instant::now());
                }
            }
            if ui
                .add_enabled(
                    app.dialog.gif.playing,
                    egui::Button::new(format!("\u{23F9}  {}", app.i18n.t("gif-stop", &[])))
                        .min_size(Vec2::new(90.0, 28.0)),
                )
                .clicked()
            {
                app.dialog.gif.playing = false;
                app.dialog.gif.play_started = None;
            }
        });
    });

    ui.add_space(6.0);

    // Export button — right-aligned.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui
            .add_enabled(
                !app.dialog.gif.frames.is_empty(),
                egui::Button::new(
                    RichText::new(format!(
                        "  \u{1F4E4}  {}  ",
                        app.i18n.t("gif-export", &[])
                    ))
                    .size(14.0)
                    .color(Color32::WHITE),
                )
                .min_size(Vec2::new(160.0, 32.0))
                .fill(super::theme::ACCENT),
            )
            .clicked()
        {
            export(app);
        }
    });
}

fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 18.0), Sense::hover());
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(1),
            super::theme::ACCENT,
        );
        ui.label(RichText::new(text).size(14.0).strong());
    });
    ui.add_space(2.0);
}

fn current_preview_frame(gif: &GifTimelineState) -> Option<usize> {
    if gif.frames.is_empty() {
        return None;
    }
    if gif.playing {
        if let Some(start) = gif.play_started {
            let delay = gif.delay_ms.max(1) as u128;
            let per_cycle = delay * gif.frames.len() as u128;
            let elapsed = start.elapsed().as_millis();
            let phase = if gif.loop_infinite {
                elapsed % per_cycle
            } else {
                elapsed.min(per_cycle.saturating_sub(1))
            };
            let idx = (phase / delay) as usize;
            return Some(idx.min(gif.frames.len() - 1));
        }
    }
    gif.selected.or(Some(0))
}

fn pick_frames(app: &mut YImageApp) {
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

/// Load and cache a thumbnail texture at THUMB_MAX_DIM for the timeline strip.
fn ensure_thumb_texture(ctx: &egui::Context, path: &std::path::Path) -> Option<TextureHandle> {
    let img = crate::io::load::load_image(path).ok()?;
    let max = THUMB_MAX_DIM;
    let (tw, th) = if img.width() <= max && img.height() <= max {
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
        format!("gif-thumb:{}", path.display()),
        ColorImage {
            size,
            source_size: egui::vec2(size[0] as f32, size[1] as f32),
            pixels,
        },
        TextureOptions::LINEAR,
    ))
}

/// Load and cache a high-resolution preview texture (PREVIEW_MAX_DIM) for step 4.
/// Only one frame is cached at a time; switching frames evicts the old cache entry.
fn ensure_preview_texture(
    ctx: &egui::Context,
    state: &mut GifTimelineState,
    path: &PathBuf,
) -> Option<TextureHandle> {
    if let Some((cached_path, tex)) = &state.preview_tex {
        if cached_path == path {
            return Some(tex.clone());
        }
    }
    let img = crate::io::load::load_image(path).ok()?;
    let max = PREVIEW_MAX_DIM;
    let (tw, th) = if img.width() <= max && img.height() <= max {
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
    };
    let large = crate::ops::resize::resize_rgba(
        &img,
        tw.max(1),
        th.max(1),
        crate::ops::resize::Filter::Lanczos3,
    )
    .ok()?;
    let size = [large.width() as usize, large.height() as usize];
    let pixels: Vec<Color32> = large
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    let tex = ctx.load_texture(
        format!("gif-preview:{}", path.display()),
        ColorImage {
            size,
            source_size: egui::vec2(size[0] as f32, size[1] as f32),
            pixels,
        },
        TextureOptions::LINEAR,
    );
    state.preview_tex = Some((path.clone(), tex.clone()));
    Some(tex)
}
