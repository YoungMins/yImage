// Central image viewer. Holds zoom/pan state, builds (and caches) the egui
// texture from the current Document, and dispatches pointer events to the
// active editing tool.

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureOptions, Vec2};

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
                            &[
                                "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "avif",
                            ],
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
                let tex = ctx.load_texture("yimage_doc", color_img, TextureOptions::LINEAR);
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
            app.viewer.zoom = sx.min(sy).clamp(0.05, 1.0);
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

        // Draw overlays (brush cursor, shape preview) on top of the image.
        draw_overlays(ctx, ui, app, &response, rect, img_size);

        // Keyboard navigation and shortcuts.
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

fn handle_tool_input(app: &mut YImageApp, response: &egui::Response, rect: Rect, img_size: Vec2) {
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
        ToolKind::Text => {
            if response.clicked_by(egui::PointerButton::Primary)
                && !app.dialog.text.content.is_empty()
            {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        doc.push_undo();
                        app.dialog.text.stamp(&mut doc.image, img_pos.0, img_pos.1);
                        app.texture_dirty = true;
                    }
                }
            }
        }
        ToolKind::Shape => {
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        app.dialog.shape_start = Some(img_pos);
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let (Some(start), Some(end)) = (
                    app.dialog.shape_start,
                    response.interact_pointer_pos().and_then(screen_to_image),
                ) {
                    doc.push_undo();
                    app.dialog.shape.commit(&mut doc.image, start, end);
                    app.texture_dirty = true;
                }
                app.dialog.shape_start = None;
            }
        }
        ToolKind::ObjectRemove => {
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

/// Draw ephemeral overlays (brush outline, shape drag preview) on top of the
/// already-painted image.
fn draw_overlays(
    ctx: &egui::Context,
    ui: &egui::Ui,
    app: &YImageApp,
    response: &egui::Response,
    rect: Rect,
    img_size: Vec2,
) {
    let painter = ui.painter_at(rect);

    // Brush cursor preview.
    if app.tool.has_brush_preview() && response.hovered() {
        if let Some(pos) = ctx.pointer_hover_pos() {
            let radius_img = match app.tool {
                ToolKind::Draw => app.dialog.brush.radius,
                ToolKind::Mosaic => app.dialog.mosaic.block_size as f32 * 0.5,
                ToolKind::ObjectRemove => 20.0,
                _ => 0.0,
            };
            let scale = rect.width() / img_size.x.max(1.0);
            let radius_screen = (radius_img * scale).max(2.0);
            let outline = Color32::from_rgb(0x00, 0x78, 0xD4);
            painter.circle_stroke(pos, radius_screen, Stroke::new(1.5, outline));
            painter.circle_stroke(pos, radius_screen, Stroke::new(0.6, Color32::WHITE));
            // Center cross.
            painter.line_segment(
                [pos - Vec2::new(3.0, 0.0), pos + Vec2::new(3.0, 0.0)],
                Stroke::new(1.0, Color32::WHITE),
            );
            painter.line_segment(
                [pos - Vec2::new(0.0, 3.0), pos + Vec2::new(0.0, 3.0)],
                Stroke::new(1.0, Color32::WHITE),
            );
        }
    }

    // Shape drag preview.
    if app.tool == ToolKind::Shape {
        if let (Some(start), Some(hover)) = (app.dialog.shape_start, ctx.pointer_hover_pos()) {
            let scale = rect.width() / img_size.x.max(1.0);
            let start_screen = rect.min + Vec2::new(start.0 * scale, start.1 * scale);
            let stroke = Stroke::new(
                2.0,
                Color32::from_rgba_unmultiplied(
                    app.dialog.shape.color[0],
                    app.dialog.shape.color[1],
                    app.dialog.shape.color[2],
                    180,
                ),
            );
            use crate::tools::draw::ShapeKind;
            match app.dialog.shape.kind {
                ShapeKind::Rect | ShapeKind::RectFilled => {
                    let r = Rect::from_two_pos(start_screen, hover);
                    painter.rect_stroke(
                        r,
                        egui::CornerRadius::ZERO,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
                ShapeKind::Ellipse | ShapeKind::EllipseFilled => {
                    let r = Rect::from_two_pos(start_screen, hover);
                    // egui has no ellipse primitive; approximate with circle
                    // using max-radius for a quick visual aid.
                    let center = r.center();
                    let rx = r.width().abs() * 0.5;
                    let ry = r.height().abs() * 0.5;
                    let n = 48;
                    let mut pts = Vec::with_capacity(n + 1);
                    for i in 0..=n {
                        let a = i as f32 * std::f32::consts::TAU / n as f32;
                        pts.push(center + Vec2::new(rx * a.cos(), ry * a.sin()));
                    }
                    for w in pts.windows(2) {
                        painter.line_segment([w[0], w[1]], stroke);
                    }
                }
                ShapeKind::Line | ShapeKind::Arrow => {
                    painter.line_segment([start_screen, hover], stroke);
                }
            }
        }
    }

    // Mosaic selection preview (rectangle being dragged).
    if app.tool == ToolKind::Mosaic {
        if let (Some(start), Some(hover)) = (app.dialog.mosaic_start, ctx.pointer_hover_pos()) {
            let scale = rect.width() / img_size.x.max(1.0);
            let start_screen = rect.min + Vec2::new(start.0 * scale, start.1 * scale);
            let r = Rect::from_two_pos(start_screen, hover);
            painter.rect_stroke(
                r,
                egui::CornerRadius::ZERO,
                Stroke::new(1.5, Color32::from_rgb(0x00, 0x78, 0xD4)),
                egui::StrokeKind::Middle,
            );
        }
    }

    // Object-removal mask: visualise the painted mask with a red overlay.
    if app.tool == ToolKind::ObjectRemove {
        if let Some(mask) = app.dialog.obj_mask.as_ref() {
            // Coarse sampled indicator — full-resolution mask rendering would
            // require uploading a new texture every frame.
            let scale = rect.width() / img_size.x.max(1.0);
            let sample_step = 8u32.max((mask.width() / 256).max(1));
            let dot = Color32::from_rgba_unmultiplied(0xE0, 0x20, 0x20, 140);
            for y in (0..mask.height()).step_by(sample_step as usize) {
                for x in (0..mask.width()).step_by(sample_step as usize) {
                    if mask.get_pixel(x, y).0[0] > 127 {
                        let p = rect.min + Vec2::new(x as f32 * scale, y as f32 * scale);
                        painter.circle_filled(p, (sample_step as f32 * scale * 0.5).max(1.0), dot);
                    }
                }
            }
        }
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
