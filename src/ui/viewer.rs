// Central image viewer. Holds zoom/pan state, builds (and caches) the egui
// texture from the current Document, and dispatches pointer events to the
// active editing tool.

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureOptions, Vec2};

use crate::app::YImageApp;
use crate::tools::ToolKind;

use super::theme;

#[derive(Default)]
pub struct ViewerState {
    pub zoom: f32,
    pub offset: Vec2,
    pub reset_view: bool,
    checker_tex: Option<egui::TextureHandle>,
}

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.tabs.is_empty() {
            show_welcome(ui, app);
            return;
        }

        let idx = app.active_tab;
        if idx >= app.tabs.len() {
            return;
        }

        // Rebuild the texture if the document changed.
        if app.tabs[idx].texture_dirty || app.tabs[idx].texture.is_none() {
            let doc = &app.tabs[idx].doc;
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
                format!("yimage_tab_{}", app.tabs[idx].id),
                color_img,
                TextureOptions::LINEAR,
            );
            app.tabs[idx].texture = Some(tex);
            app.tabs[idx].texture_dirty = false;
        }

        let tex_id = match app.tabs[idx].texture.as_ref() {
            Some(tex) => tex.id(),
            None => return,
        };

        let avail = ui.available_rect_before_wrap();
        let img_size = Vec2::new(
            app.tabs[idx].doc.width() as f32,
            app.tabs[idx].doc.height() as f32,
        );

        // Initial fit or explicit reset.
        if app.tabs[idx].viewer.zoom == 0.0 || app.tabs[idx].viewer.reset_view {
            let sx = avail.width() / img_size.x;
            let sy = avail.height() / img_size.y;
            app.tabs[idx].viewer.zoom = sx.min(sy).clamp(0.05, 1.0);
            app.tabs[idx].viewer.offset = Vec2::ZERO;
            app.tabs[idx].viewer.reset_view = false;
        }

        let display_size = img_size * app.tabs[idx].viewer.zoom;
        let center = avail.center() + app.tabs[idx].viewer.offset;
        let rect = Rect::from_center_size(center, display_size);

        let response = ui.allocate_rect(avail, Sense::click_and_drag());

        // Pan with middle mouse or right mouse drag.
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            app.tabs[idx].viewer.offset += response.drag_delta();
        }

        // Zoom with mouse wheel, anchored on the cursor.
        ctx.input(|i| {
            let scroll = i.smooth_scroll_delta.y;
            if scroll != 0.0 && response.hovered() {
                let factor = (scroll * 0.002).exp();
                let old_zoom = app.tabs[idx].viewer.zoom;
                let new_zoom = (old_zoom * factor).clamp(0.02, 32.0);
                if let Some(pos) = i.pointer.hover_pos() {
                    let delta = pos - center;
                    app.tabs[idx].viewer.offset += delta * (1.0 - new_zoom / old_zoom);
                }
                app.tabs[idx].viewer.zoom = new_zoom;
            }
        });

        // Checkerboard transparency background behind the image.
        draw_checkerboard(ctx, ui, &mut app.tabs[idx].viewer, rect, app.settings.theme_dark);

        // Draw the image texture.
        egui::Image::new((tex_id, display_size))
            .fit_to_exact_size(display_size)
            .paint_at(ui, rect);

        // Dispatch pointer events to the active tool.
        handle_tool_input(app, &response, rect, img_size);

        // Draw overlays (brush cursor, shape preview) on top of the image.
        draw_overlays(ctx, ui, app, &response, rect, img_size);

        // Keyboard navigation and shortcuts.
        handle_keyboard(ctx, app);
    });
}

fn handle_keyboard(ctx: &egui::Context, app: &mut YImageApp) {
    ctx.input(|i| {
        if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown) {
            app.navigate(1);
        }
        if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp) {
            app.navigate(-1);
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                if tab.doc.undo() {
                    tab.texture_dirty = true;
                }
            }
        }
        if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z) {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                if tab.doc.redo() {
                    tab.texture_dirty = true;
                }
            }
        }
        if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
            app.save_current();
        }
        if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
            app.dialog.save_dialog_open = true;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::W) {
            if !app.tabs.is_empty() {
                app.close_tab(app.active_tab);
            }
        }
    });
}

// ── Welcome screen ─────────────────────────────────────────────────

fn show_welcome(ui: &mut egui::Ui, app: &mut YImageApp) {
    let avail = ui.available_size();
    let card_width = 420.0_f32.min(avail.x - 40.0);

    ui.allocate_ui_at_rect(
        Rect::from_center_size(
            ui.max_rect().center(),
            Vec2::new(card_width, avail.y),
        ),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(avail.y * 0.22);

                // Wordmark.
                ui.label(
                    egui::RichText::new("yImage")
                        .size(theme::FONT_DISPLAY)
                        .color(theme::ACCENT),
                );
                ui.add_space(theme::SPACE_XS);
                ui.label(
                    egui::RichText::new(app.i18n.t("welcome-tagline", &[]))
                        .size(theme::FONT_BODY)
                        .color(if app.settings.theme_dark {
                            theme::TEXT_SECONDARY_DARK
                        } else {
                            theme::TEXT_SECONDARY_LIGHT
                        }),
                );

                ui.add_space(theme::SPACE_XL);

                // Card with open button + recent files.
                let frame = theme::card_frame(app.settings.theme_dark);
                frame.show(ui, |ui| {
                    ui.set_min_width(card_width - 40.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(theme::SPACE_SM);

                        // Open button.
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "\u{1F4C2}  {}",
                                        app.i18n.t("action-open", &[])
                                    ))
                                    .size(14.0)
                                    .color(Color32::WHITE),
                                )
                                .min_size(Vec2::new(180.0, 40.0))
                                .fill(theme::ACCENT)
                                .corner_radius(egui::CornerRadius::same(10)),
                            )
                            .clicked()
                        {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter(
                                    "images",
                                    &[
                                        "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif",
                                        "tiff", "avif",
                                    ],
                                )
                                .pick_file()
                            {
                                app.open_path(&p, true);
                            }
                        }

                        ui.add_space(theme::SPACE_SM);
                        ui.label(
                            egui::RichText::new(app.i18n.t("welcome-open", &[]))
                                .size(theme::FONT_CAPTION)
                                .color(if app.settings.theme_dark {
                                    theme::TEXT_SECONDARY_DARK
                                } else {
                                    theme::TEXT_SECONDARY_LIGHT
                                }),
                        );

                        // Recent files section.
                        let recents: Vec<_> = app
                            .settings
                            .recent_files
                            .iter()
                            .filter(|p| p.exists())
                            .take(5)
                            .cloned()
                            .collect();

                        if !recents.is_empty() {
                            ui.add_space(theme::SPACE_MD);
                            let divider_color = if app.settings.theme_dark {
                                theme::DIVIDER_DARK
                            } else {
                                theme::DIVIDER_LIGHT
                            };
                            let avail_w = ui.available_width();
                            let (line_rect, _) =
                                ui.allocate_exact_size(Vec2::new(avail_w, 1.0), Sense::hover());
                            ui.painter().line_segment(
                                [line_rect.left_center(), line_rect.right_center()],
                                Stroke::new(0.5, divider_color),
                            );
                            ui.add_space(theme::SPACE_MD);

                            ui.label(
                                egui::RichText::new(app.i18n.t("welcome-recent", &[]))
                                    .size(theme::FONT_CAPTION)
                                    .strong(),
                            );
                            ui.add_space(theme::SPACE_XS);

                            let mut open_path = None;
                            for path in &recents {
                                let name = path
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("?");
                                let folder = path
                                    .parent()
                                    .and_then(|p| p.file_name())
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("");

                                let r = ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(name)
                                            .size(theme::FONT_BODY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(folder)
                                                    .size(theme::FONT_CAPTION)
                                                    .color(if app.settings.theme_dark {
                                                        theme::TEXT_SECONDARY_DARK
                                                    } else {
                                                        theme::TEXT_SECONDARY_LIGHT
                                                    }),
                                            );
                                        },
                                    );
                                });
                                let resp = r.response.interact(Sense::click());
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        resp.rect.expand(2.0),
                                        egui::CornerRadius::same(4),
                                        theme::ACCENT.linear_multiply(0.1),
                                    );
                                }
                                if resp.clicked() {
                                    open_path = Some(path.clone());
                                }
                            }
                            if let Some(p) = open_path {
                                app.open_path(&p, true);
                            }
                        }

                        ui.add_space(theme::SPACE_SM);
                    });
                });

                ui.add_space(theme::SPACE_LG);
                ui.label(
                    egui::RichText::new(app.i18n.t("welcome-shortcut-hint", &[]))
                        .size(theme::FONT_TINY)
                        .monospace()
                        .color(if app.settings.theme_dark {
                            theme::TEXT_SECONDARY_DARK
                        } else {
                            theme::TEXT_SECONDARY_LIGHT
                        }),
                );
            });
        },
    );
}

// ── Checkerboard ───────────────────────────────────────────────────

fn draw_checkerboard(
    ctx: &egui::Context,
    ui: &egui::Ui,
    viewer: &mut ViewerState,
    rect: Rect,
    dark: bool,
) {
    const CELL: usize = 8;
    if viewer.checker_tex.is_none() {
        let (a, b) = if dark {
            (Color32::from_gray(0x30), Color32::from_gray(0x40))
        } else {
            (Color32::from_gray(0xE0), Color32::from_gray(0xFF))
        };
        let dim = CELL * 2;
        let mut pixels = vec![a; dim * dim];
        for y in 0..dim {
            for x in 0..dim {
                let checker = ((x / CELL) + (y / CELL)) % 2 == 0;
                pixels[y * dim + x] = if checker { a } else { b };
            }
        }
        let img = ColorImage {
            size: [dim, dim],
            source_size: egui::vec2(dim as f32, dim as f32),
            pixels,
        };
        let mut opts = TextureOptions::NEAREST;
        opts.wrap_mode = egui::TextureWrapMode::Repeat;
        viewer.checker_tex = Some(ctx.load_texture("yimage_checker", img, opts));
    }

    if let Some(tex) = &viewer.checker_tex {
        let clipped = rect.intersect(ui.clip_rect());
        if clipped.width() > 0.0 && clipped.height() > 0.0 {
            let tile = (CELL * 2) as f32;
            let uv = Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(clipped.width() / tile, clipped.height() / tile),
            );
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_rect_with_uv(clipped, uv, Color32::WHITE);
            ui.painter().add(egui::Shape::mesh(mesh));
        }
    }
}

// ── Tool input ─────────────────────────────────────────────────────

fn handle_tool_input(app: &mut YImageApp, response: &egui::Response, rect: Rect, img_size: Vec2) {
    if app.tabs.is_empty() || app.active_tab >= app.tabs.len() {
        return;
    }
    let idx = app.active_tab;

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
                app.tabs[idx].doc.push_undo();
                app.dialog.brush.begin();
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(img_pos) = screen_to_image(pos) {
                        app.dialog.brush.stroke(&mut app.tabs[idx].doc.image, img_pos);
                        app.tabs[idx].texture_dirty = true;
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
                        app.tabs[idx].doc.push_undo();
                        crate::tools::mosaic::apply_mosaic(
                            &mut app.tabs[idx].doc.image,
                            (x0, y0, x1 - x0, y1 - y0),
                            app.dialog.mosaic.block_size,
                        );
                        app.tabs[idx].texture_dirty = true;
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
                        app.tabs[idx].doc.push_undo();
                        app.dialog
                            .text
                            .stamp(&mut app.tabs[idx].doc.image, img_pos.0, img_pos.1);
                        app.tabs[idx].texture_dirty = true;
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
                    app.tabs[idx].doc.push_undo();
                    app.dialog
                        .shape
                        .commit(&mut app.tabs[idx].doc.image, start, end);
                    app.tabs[idx].texture_dirty = true;
                }
                app.dialog.shape_start = None;
            }
        }
        ToolKind::ObjectRemove => {
            if response.drag_started_by(egui::PointerButton::Primary) {
                let (w, h) = (
                    app.tabs[idx].doc.width(),
                    app.tabs[idx].doc.height(),
                );
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
