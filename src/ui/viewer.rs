// Central image viewer. Holds zoom/pan state, builds (and caches) the egui
// texture from the current Document, and dispatches pointer events to the
// active editing tool.

use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureOptions, Vec2};

use crate::app::YImageApp;
use crate::tools::ToolKind;

#[derive(Default)]
pub struct ViewerState {
    pub zoom: f32,
    pub offset: Vec2,
    pub reset_view: bool,
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.doc.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.heading("yImage");
                ui.label(app.i18n.t("welcome-open", &[]));
                if ui.button(app.i18n.t("action-open", &[])).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter(
                            "images",
                            &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "avif"],
                        )
                        .pick_file()
                    {
                        app.open_path(&p);
                    }
                }
            });
            return;
        }

        // Rebuild the texture if the document changed.
        if app.texture_dirty || app.texture.is_none() {
            if let Some(doc) = &app.doc {
                let size = [doc.width() as usize, doc.height() as usize];
                let pixels: Vec<Color32> = doc
                    .image
                    .pixels()
                    .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                let color_img = ColorImage {
                    size,
                    source_size: egui::vec2(size[0] as f32, size[1] as f32),
                    pixels,
                };
                let tex = ctx.load_texture(
                    "yimage_doc",
                    color_img,
                    TextureOptions::LINEAR,
                );
                app.texture = Some(tex);
                app.texture_dirty = false;
            }
        }

        let Some(tex) = app.texture.as_ref() else {
            return;
        };

        let avail = ui.available_rect_before_wrap();
        let doc = app.doc.as_ref().unwrap();
        let img_size = Vec2::new(doc.width() as f32, doc.height() as f32);

        // Initial fit or explicit reset.
        if app.viewer.zoom == 0.0 || app.viewer.reset_view {
            let sx = avail.width() / img_size.x;
            let sy = avail.height() / img_size.y;
            app.viewer.zoom = sx.min(sy).min(1.0).max(0.05);
            app.viewer.offset = Vec2::ZERO;
            app.viewer.reset_view = false;
        }

        let display_size = img_size * app.viewer.zoom;
        let center = avail.center() + app.viewer.offset;
        let rect = Rect::from_center_size(center, display_size);

        let response = ui.allocate_rect(avail, Sense::click_and_drag());

        // Pan with middle mouse or right mouse drag.
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            app.viewer.offset += response.drag_delta();
        }

        // Zoom with mouse wheel, anchored on the cursor.
        ctx.input(|i| {
            let scroll = i.smooth_scroll_delta.y;
            if scroll != 0.0 && response.hovered() {
                let factor = (scroll * 0.002).exp();
                let old_zoom = app.viewer.zoom;
                let new_zoom = (old_zoom * factor).clamp(0.02, 32.0);
                if let Some(pos) = i.pointer.hover_pos() {
                    // Keep the point under the cursor stable.
                    let delta = pos - center;
                    app.viewer.offset += delta * (1.0 - new_zoom / old_zoom);
                }
                app.viewer.zoom = new_zoom;
            }
        });

        // Draw the image texture.
        egui::Image::new((tex.id(), display_size))
            .fit_to_exact_size(display_size)
            .paint_at(ui, rect);

        // Dispatch pointer events to the active tool.
        handle_tool_input(app, &response, rect, img_size);

        // Click navigation via left/right arrows handled in input.
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown) {
                app.navigate(1);
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp) {
                app.navigate(-1);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                if let Some(doc) = app.doc.as_mut() {
                    if doc.undo() {
                        app.texture_dirty = true;
                    }
                }
            }
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z) {
                if let Some(doc) = app.doc.as_mut() {
                    if doc.redo() {
                        app.texture_dirty = true;
                    }
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                app.dialog.save_dialog_open = true;
            }
        });
    });
}

fn handle_tool_input(
    app: &mut YImageApp,
    response: &egui::Response,
    rect: Rect,
    img_size: Vec2,
) {
    let Some(doc) = app.doc.as_mut() else { return };

    let screen_to_image = |pos: Pos2| -> Option<(f32, f32)> {
        let rel = pos - rect.min;
        let u = rel.x / rect.width();
        let v = rel.y / rect.height();
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }
        Some((u * img_size.x, v * img_size.y))
    };

    match app.tool {
        ToolKind::Draw => {
            if response.drag_started_by(egui::PointerButton::Primary) {
                doc.push_undo();
                app.dialog.brush.begin();
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        app.dialog.brush.stroke(&mut doc.image, img_pos);
                        app.texture_dirty = true;
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                app.dialog.brush.end();
            }
        }
        ToolKind::Mosaic => {
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        app.dialog.mosaic_start = Some(img_pos);
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let (Some(start), Some(end)) = (
                    app.dialog.mosaic_start,
                    response.interact_pointer_pos().and_then(screen_to_image),
                ) {
                    let x0 = start.0.min(end.0).max(0.0) as u32;
                    let y0 = start.1.min(end.1).max(0.0) as u32;
                    let x1 = start.0.max(end.0) as u32;
                    let y1 = start.1.max(end.1) as u32;
                    if x1 > x0 && y1 > y0 {
                        doc.push_undo();
                        crate::tools::mosaic::apply_mosaic(
                            &mut doc.image,
                            (x0, y0, x1 - x0, y1 - y0),
                            app.dialog.mosaic.block_size,
                        );
                        app.texture_dirty = true;
                    }
                }
                app.dialog.mosaic_start = None;
            }
        }
        ToolKind::ObjectRemove => {
            // User brushes onto a mask overlay while the tool is active.
            if response.drag_started_by(egui::PointerButton::Primary) {
                let (w, h) = (doc.width(), doc.height());
                if app.dialog.obj_mask.is_none() {
                    app.dialog.obj_mask = Some(image::GrayImage::new(w, h));
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        if let Some(mask) = app.dialog.obj_mask.as_mut() {
                            stamp_mask(mask, img_pos, 20);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn stamp_mask(mask: &mut image::GrayImage, pos: (f32, f32), radius: i32) {
    let w = mask.width() as i32;
    let h = mask.height() as i32;
    let cx = pos.0 as i32;
    let cy = pos.1 as i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x >= 0 && y >= 0 && x < w && y < h {
                mask.put_pixel(x as u32, y as u32, image::Luma([255]));
            }
        }
    }
}
