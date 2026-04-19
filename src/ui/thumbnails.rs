// Bottom filmstrip of folder thumbnails.
//
// A horizontal strip of small tiles representing every image in the current
// folder so the user can jump to siblings without a file picker. Thumbnails
// are decoded lazily on rayon workers and cached in a HashMap keyed by path.
// Toggled via View → Thumbnail Panel (default: off to give the canvas the full
// viewport).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, ColorImage, CornerRadius, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use parking_lot::Mutex;

use crate::app::YImageApp;
use crate::ui::theme;

pub const THUMB_MAX_DIM: u32 = 96;
const STRIP_HEIGHT: f32 = 92.0;
const TILE_SIZE: f32 = 64.0;

#[derive(Default)]
pub struct Thumbnails {
    pub cache: Arc<Mutex<HashMap<PathBuf, TextureHandle>>>,
    pub pending: Arc<Mutex<HashMap<PathBuf, ()>>>,
    pub visible: bool,
}

impl Thumbnails {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
            pending: Default::default(),
            visible: false,
        }
    }
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if !app.thumbs.visible {
        return;
    }

    let dark = ctx.style().visuals.dark_mode;
    let strip_frame = egui::Frame::none()
        .fill(if dark { theme::GRADIENT_BOT_DARK } else { theme::GRADIENT_BOT_LIGHT })
        .stroke(egui::Stroke::NONE);

    egui::TopBottomPanel::bottom("thumbnails_strip")
        .exact_height(STRIP_HEIGHT)
        .frame(strip_frame)
        .show_separator_line(false)
        .show(ctx, |ui| {
            let entries = app.folder_entries.lock().clone();
            if entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(STRIP_HEIGHT * 0.35);
                    ui.weak(
                        egui::RichText::new(app.i18n.t("thumbs-empty", &[]))
                            .size(theme::FONT_CAPTION),
                    );
                });
                return;
            }

            let current_path = app.active_doc().and_then(|d| d.path.as_ref()).cloned();
            let mut nav_target: Option<PathBuf> = None;

            egui::ScrollArea::horizontal()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        for entry in &entries {
                            let is_current = current_path.as_ref() == Some(entry);
                            let tex = ensure_thumbnail(ctx, app, entry);

                            let (alloc_rect, resp) = ui.allocate_exact_size(
                                Vec2::splat(TILE_SIZE + 6.0),
                                Sense::click(),
                            );

                            // Subtle scale-up on hover — animated so the tile
                            // doesn't jump as the pointer enters/leaves.
                            let hover_id = ui.id().with(("thumb_hover", entry));
                            let hover_t = ctx.animate_bool_with_time(
                                hover_id,
                                resp.hovered(),
                                0.12,
                            );
                            let grow = 2.0 * hover_t;
                            let rect = alloc_rect.expand(grow);

                            // Highlight the active tile with an accent frame.
                            let stroke = if is_current {
                                Stroke::new(2.0, theme::ACCENT)
                            } else if resp.hovered() {
                                Stroke::new(
                                    1.0,
                                    ui.visuals().widgets.hovered.fg_stroke.color,
                                )
                            } else {
                                Stroke::NONE
                            };
                            let fill = if is_current {
                                theme::ACCENT.linear_multiply(0.15)
                            } else if resp.hovered() {
                                ui.visuals().widgets.hovered.weak_bg_fill
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, CornerRadius::same(8), fill);
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(8),
                                stroke,
                                egui::StrokeKind::Inside,
                            );

                            let inner = rect.shrink(3.0);
                            match tex {
                                Some(t) => {
                                    ui.painter().image(
                                        t.id(),
                                        inner,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                }
                                None => {
                                    let t = ctx.input(|i| i.time) as f32;
                                    let pulse: u8 = (20.0 + 20.0 * (t * 2.0).sin().abs()) as u8;
                                    let base: u8 = if app.settings.theme_dark { 40 } else { 200 };
                                    ui.painter().rect_filled(
                                        inner,
                                        CornerRadius::same(5),
                                        Color32::from_gray(base.saturating_add(pulse)),
                                    );
                                }
                            }

                            if let Some(name) = entry.file_name().and_then(|s| s.to_str()) {
                                resp.clone().on_hover_text(name);
                            }
                            if resp.clicked() {
                                nav_target = Some(entry.clone());
                            }
                        }
                        ui.add_space(4.0);
                    });
                });

            if let Some(p) = nav_target {
                let is_dirty = app.tabs.get(app.active_tab).map_or(false, |t| t.doc.dirty);
                app.open_path(&p, is_dirty);
            }
        });
}

pub(crate) fn ensure_thumbnail(
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
