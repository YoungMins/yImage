// Left-hand PowerPoint-style thumbnail strip.
//
// Shows every image in the current folder as a small thumbnail so the user
// can jump around without opening a file picker. Thumbnails are generated
// lazily on background rayon workers and cached in an LRU-ish HashMap keyed
// by path. The panel is resizable so users can hide it when they want more
// screen real estate for the viewer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Vec2};
use parking_lot::Mutex;

use crate::app::YImageApp;

pub const THUMB_MAX_DIM: u32 = 180;

#[derive(Default)]
pub struct Thumbnails {
    /// Finished thumbnails keyed by path → egui texture.
    pub cache: Arc<Mutex<HashMap<PathBuf, TextureHandle>>>,
    /// Paths that are currently being decoded on a worker so we don't
    /// schedule duplicate jobs.
    pub pending: Arc<Mutex<HashMap<PathBuf, ()>>>,
    /// User-controlled visibility.
    pub visible: bool,
}

impl Thumbnails {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
            pending: Default::default(),
            visible: true,
        }
    }
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if !app.thumbs.visible {
        return;
    }
    egui::SidePanel::left("thumbnails")
        .resizable(true)
        .default_width(200.0)
        .min_width(120.0)
        .max_width(320.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(app.i18n.t("thumbs-title", &[]));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("⟳")
                        .on_hover_text(app.i18n.t("thumbs-refresh", &[]))
                        .clicked()
                    {
                        if let Some(doc) = &app.doc {
                            if let Some(path) = &doc.path {
                                let p = path.clone();
                                app.scan_folder_now(&p);
                            }
                        }
                    }
                });
            });
            ui.separator();

            let entries = app.folder_entries.lock().clone();
            if entries.is_empty() {
                ui.label(app.i18n.t("thumbs-empty", &[]));
                return;
            }

            let current_path = app.doc.as_ref().and_then(|d| d.path.as_ref()).cloned();

            let mut nav_target: Option<PathBuf> = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for entry in &entries {
                        let is_current = current_path.as_ref() == Some(entry);
                        let tex = ensure_thumbnail(ctx, app, entry);

                        let frame = egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::same(4))
                            .corner_radius(egui::CornerRadius::same(6))
                            .fill(if is_current {
                                super::theme::ACCENT.linear_multiply(0.25)
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(if is_current {
                                egui::Stroke::new(1.0, super::theme::ACCENT)
                            } else {
                                egui::Stroke::NONE
                            });

                        let r = frame
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    let size = Vec2::splat(THUMB_MAX_DIM as f32 * 0.9);
                                    match tex {
                                        Some(t) => {
                                            ui.add(
                                                egui::Image::new((t.id(), size))
                                                    .fit_to_exact_size(size),
                                            );
                                        }
                                        None => {
                                            let (rect, _) =
                                                ui.allocate_exact_size(size, egui::Sense::hover());
                                            ui.painter().rect_filled(
                                                rect,
                                                egui::CornerRadius::same(4),
                                                Color32::from_rgb(0x30, 0x30, 0x30),
                                            );
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "…",
                                                egui::FontId::proportional(18.0),
                                                Color32::from_gray(160),
                                            );
                                        }
                                    }
                                    let name =
                                        entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(name).small())
                                            .truncate(),
                                    );
                                });
                            })
                            .response;

                        let r = r.interact(egui::Sense::click());
                        if r.clicked() {
                            nav_target = Some(entry.clone());
                        }
                    }
                });

            if let Some(p) = nav_target {
                app.open_path(&p);
            }
        });
}

/// Return the cached thumbnail for `path`, kicking off a background decode
/// if one hasn't been started yet.
fn ensure_thumbnail(
    ctx: &egui::Context,
    app: &YImageApp,
    path: &std::path::Path,
) -> Option<TextureHandle> {
    {
        let cache = app.thumbs.cache.lock();
        if let Some(t) = cache.get(path) {
            return Some(t.clone());
        }
    }

    // Already scheduled?
    {
        let mut pending = app.thumbs.pending.lock();
        if pending.contains_key(path) {
            return None;
        }
        pending.insert(path.to_path_buf(), ());
    }

    let cache = app.thumbs.cache.clone();
    let pending = app.thumbs.pending.clone();
    let ctx_clone = ctx.clone();
    let path_owned = path.to_path_buf();

    rayon::spawn(move || {
        let Ok(img) = crate::io::load::load_image(&path_owned) else {
            pending.lock().remove(&path_owned);
            return;
        };
        let (tw, th) = fit_within(img.width(), img.height(), THUMB_MAX_DIM);
        let small = match crate::ops::resize::resize_rgba(
            &img,
            tw,
            th,
            crate::ops::resize::Filter::Bilinear,
        ) {
            Ok(s) => s,
            Err(_) => {
                pending.lock().remove(&path_owned);
                return;
            }
        };
        let size = [tw as usize, th as usize];
        let pixels: Vec<Color32> = small
            .pixels()
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        let color_image = ColorImage {
            size,
            source_size: egui::vec2(size[0] as f32, size[1] as f32),
            pixels,
        };
        let tex = ctx_clone.load_texture(
            format!("thumb:{}", path_owned.display()),
            color_image,
            TextureOptions::LINEAR,
        );
        cache.lock().insert(path_owned.clone(), tex);
        pending.lock().remove(&path_owned);
        ctx_clone.request_repaint();
    });

    None
}

fn fit_within(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    if w <= max_dim && h <= max_dim {
        return (w.max(1), h.max(1));
    }
    let ratio = w as f32 / h as f32;
    if w >= h {
        let nw = max_dim;
        let nh = (max_dim as f32 / ratio).max(1.0) as u32;
        (nw, nh.max(1))
    } else {
        let nh = max_dim;
        let nw = (max_dim as f32 * ratio).max(1.0) as u32;
        (nw.max(1), nh)
    }
}
