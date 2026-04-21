// Central image viewer. Holds zoom/pan state, builds (and caches) the egui
// texture from the current Document, and dispatches pointer events to the
// active editing tool.

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureOptions, Vec2};

use crate::app::YImageApp;
use crate::tools::ToolKind;
use crate::ui::dialogs::CropHandle;

use super::theme;

const HANDLE_RADIUS: f32 = 5.0;
const HANDLE_GRAB_RADIUS: f32 = 12.0;
const RULER_WIDTH: f32 = 22.0;

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
            let img = &app.tabs[idx].doc.image;
            let color_img = if img.width().max(img.height()) > 8192 {
                let scale = 8192.0 / img.width().max(img.height()) as f32;
                let dw = (img.width() as f32 * scale).max(1.0) as u32;
                let dh = (img.height() as f32 * scale).max(1.0) as u32;
                match crate::ops::resize::resize_rgba(
                    img,
                    dw,
                    dh,
                    crate::ops::resize::Filter::Bilinear,
                ) {
                    Ok(small) => super::rgba_to_color_image(&small),
                    Err(_) => super::rgba_to_color_image(img),
                }
            } else {
                super::rgba_to_color_image(img)
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
    let card_width = 720.0_f32.min(avail.x - 40.0);

    ui.allocate_ui_at_rect(
        Rect::from_center_size(
            ui.max_rect().center(),
            Vec2::new(card_width, avail.y),
        ),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(avail.y * 0.14);

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
                        .min_size(Vec2::new(200.0, 44.0))
                        .fill(theme::ACCENT)
                        .corner_radius(egui::CornerRadius::same(12)),
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

                ui.add_space(theme::SPACE_XL);

                // Horizontal row of recent-file cards.
                let recents: Vec<_> = app
                    .settings
                    .recent_files
                    .iter()
                    .filter(|p| p.exists())
                    .take(6)
                    .cloned()
                    .collect();

                if !recents.is_empty() {
                    ui.label(
                        egui::RichText::new(app.i18n.t("welcome-recent", &[]))
                            .size(theme::FONT_CAPTION)
                            .strong(),
                    );
                    ui.add_space(theme::SPACE_SM);

                    let mut open_path = None;
                    let ctx = ui.ctx().clone();
                    egui::ScrollArea::horizontal()
                        .max_width(card_width)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                for path in &recents {
                                    if recent_card(ui, &ctx, app, path).clicked() {
                                        open_path = Some(path.clone());
                                    }
                                }
                            });
                        });
                    if let Some(p) = open_path {
                        app.open_path(&p, true);
                    }
                }

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

fn recent_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &YImageApp,
    path: &std::path::Path,
) -> egui::Response {
    const CARD_W: f32 = 140.0;
    const THUMB_H: f32 = 100.0;
    const LABEL_H: f32 = 34.0;
    const CARD_H: f32 = THUMB_H + LABEL_H;

    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(CARD_W, CARD_H), Sense::click());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(10);

    // Card surface.
    let surface = if app.settings.theme_dark {
        Color32::from_rgba_unmultiplied(0x2C, 0x2C, 0x2E, 235)
    } else {
        Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 230)
    };
    painter.rect_filled(rect, radius, surface);

    let stroke_color = if resp.hovered() {
        theme::ACCENT
    } else if app.settings.theme_dark {
        theme::DIVIDER_DARK
    } else {
        theme::DIVIDER_LIGHT
    };
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(if resp.hovered() { 1.5 } else { 0.5 }, stroke_color),
        egui::StrokeKind::Inside,
    );

    // Thumbnail area.
    let thumb_rect = Rect::from_min_size(rect.min, Vec2::new(CARD_W, THUMB_H));
    let thumb_bg = if app.settings.theme_dark {
        Color32::from_rgb(0x1C, 0x1C, 0x1E)
    } else {
        Color32::from_rgb(0xF0, 0xEF, 0xF5)
    };
    painter.rect_filled(
        thumb_rect,
        egui::CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 },
        thumb_bg,
    );

    if let Some(tex) = crate::ui::thumbnails::ensure_thumbnail(ctx, app, path) {
        let [tw, th] = tex.size();
        let img_ratio = tw as f32 / th.max(1) as f32;
        let box_ratio = thumb_rect.width() / thumb_rect.height();
        let inset = 8.0;
        let inner = thumb_rect.shrink(inset);
        let draw_size = if img_ratio > box_ratio {
            Vec2::new(inner.width(), inner.width() / img_ratio)
        } else {
            Vec2::new(inner.height() * img_ratio, inner.height())
        };
        let draw_rect = Rect::from_center_size(inner.center(), draw_size);
        painter.image(
            tex.id(),
            draw_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        // Pulsing placeholder while the thumbnail decodes.
        let t = ctx.input(|i| i.time) as f32;
        let alpha = ((t * 2.0).sin() * 0.5 + 0.5) * 40.0 + 20.0;
        painter.rect_filled(
            thumb_rect.shrink(16.0),
            egui::CornerRadius::same(6),
            Color32::from_black_alpha(alpha as u8),
        );
    }

    // Filename label.
    let name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("?");
    let label_rect = Rect::from_min_size(
        egui::pos2(rect.min.x, rect.min.y + THUMB_H),
        Vec2::new(CARD_W, LABEL_H),
    );
    let text_color = if app.settings.theme_dark {
        Color32::from_rgb(0xEE, 0xEE, 0xF0)
    } else {
        Color32::from_rgb(0x1C, 0x1C, 0x1E)
    };
    let elided = elide_middle(name, 18);
    painter.text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        elided,
        egui::FontId::proportional(theme::FONT_CAPTION),
        text_color,
    );

    resp.on_hover_text(name)
}

fn elide_middle(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let head = keep / 2;
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(chars[chars.len() - tail..].iter());
    out
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
        ToolKind::Crop => {
            // Auto-init crop rect to the full image.
            if app.dialog.crop_rect.is_none() {
                let (w, h) = (app.tabs[idx].doc.width(), app.tabs[idx].doc.height());
                app.dialog.crop_rect = Some((0, 0, w, h));
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(cr) = app.dialog.crop_rect {
                        app.dialog.crop_drag_handle =
                            detect_crop_handle(pos, cr, rect, img_size);
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let (Some(handle), Some(pos), Some(ref mut cr)) =
                    (app.dialog.crop_drag_handle, response.interact_pointer_pos(), app.dialog.crop_rect.as_mut())
                {
                    let rel = pos - rect.min;
                    let ix = rel.x / (rect.width() / img_size.x);
                    let iy = rel.y / (rect.height() / img_size.y);
                    let img_w = app.tabs[idx].doc.width();
                    let img_h = app.tabs[idx].doc.height();
                    update_crop_for_handle(cr, handle, ix, iy, img_w, img_h);
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                app.dialog.crop_drag_handle = None;
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
    app: &mut YImageApp,
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

    // Crop: rulers, dim outside selection, crop border, 8 transform handles.
    if app.tool == ToolKind::Crop {
        if let Some(cr) = app.dialog.crop_rect {
            let scale_x = rect.width() / img_size.x.max(1.0);
            let scale_y = rect.height() / img_size.y.max(1.0);
            let sel = Rect::from_min_size(
                rect.min + Vec2::new(cr.0 as f32 * scale_x, cr.1 as f32 * scale_y),
                Vec2::new(cr.2 as f32 * scale_x, cr.3 as f32 * scale_y),
            );

            // Dim outside the selection.
            let dim = Color32::from_black_alpha(120);
            for r in [
                Rect::from_min_max(rect.min, egui::pos2(rect.max.x, sel.min.y)),
                Rect::from_min_max(egui::pos2(rect.min.x, sel.max.y), rect.max),
                Rect::from_min_max(egui::pos2(rect.min.x, sel.min.y), egui::pos2(sel.min.x, sel.max.y)),
                Rect::from_min_max(egui::pos2(sel.max.x, sel.min.y), egui::pos2(rect.max.x, sel.max.y)),
            ] {
                if r.width() > 0.0 && r.height() > 0.0 {
                    painter.rect_filled(r, egui::CornerRadius::ZERO, dim);
                }
            }

            // Rule-of-thirds guides inside selection.
            let guide_stroke = Stroke::new(0.5, Color32::from_white_alpha(60));
            for i in 1..3 {
                let t = i as f32 / 3.0;
                let vx = sel.min.x + sel.width() * t;
                painter.line_segment([Pos2::new(vx, sel.min.y), Pos2::new(vx, sel.max.y)], guide_stroke);
                let hy = sel.min.y + sel.height() * t;
                painter.line_segment([Pos2::new(sel.min.x, hy), Pos2::new(sel.max.x, hy)], guide_stroke);
            }

            // Selection border.
            painter.rect_stroke(sel, egui::CornerRadius::ZERO, Stroke::new(1.5, Color32::from_rgb(0x00, 0x78, 0xD4)), egui::StrokeKind::Middle);
            painter.rect_stroke(sel, egui::CornerRadius::ZERO, Stroke::new(0.5, Color32::WHITE), egui::StrokeKind::Inside);

            // 8 transform handles.
            draw_crop_handles(&painter, cr, rect, img_size, app.dialog.crop_drag_handle);

            // Rulers along top and left edges of the image.
            draw_rulers(&painter, rect, img_size, cr, app.settings.theme_dark);
        }
    }

    // Object-removal mask: solid semi-transparent red overlay via GPU texture.
    if app.tool == ToolKind::ObjectRemove {
        if let Some(mask) = app.dialog.obj_mask.as_ref() {
            let w = mask.width() as usize;
            let h = mask.height() as usize;
            let pixels: Vec<Color32> = mask
                .pixels()
                .map(|p| {
                    if p.0[0] > 127 {
                        Color32::from_rgba_unmultiplied(0xE0, 0x30, 0x30, 100)
                    } else {
                        Color32::TRANSPARENT
                    }
                })
                .collect();
            let color_img = ColorImage {
                size: [w, h],
                source_size: egui::vec2(w as f32, h as f32),
                pixels,
            };
            let tex = app.dialog.obj_mask_tex.get_or_insert_with(|| {
                ctx.load_texture("obj_mask_overlay", color_img.clone(), TextureOptions::LINEAR)
            });
            tex.set(color_img, TextureOptions::LINEAR);
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
}

// ── Crop helpers ──────────────────────────────────────────────────

fn crop_handle_screen_pos(
    cr: (u32, u32, u32, u32),
    handle: CropHandle,
    rect: Rect,
    img_size: Vec2,
) -> Pos2 {
    let sx = rect.width() / img_size.x.max(1.0);
    let sy = rect.height() / img_size.y.max(1.0);
    let (x, y, w, h) = (cr.0 as f32, cr.1 as f32, cr.2 as f32, cr.3 as f32);
    let (ix, iy) = match handle {
        CropHandle::NW => (x, y),
        CropHandle::N  => (x + w * 0.5, y),
        CropHandle::NE => (x + w, y),
        CropHandle::W  => (x, y + h * 0.5),
        CropHandle::E  => (x + w, y + h * 0.5),
        CropHandle::SW => (x, y + h),
        CropHandle::S  => (x + w * 0.5, y + h),
        CropHandle::SE => (x + w, y + h),
    };
    rect.min + Vec2::new(ix * sx, iy * sy)
}

fn detect_crop_handle(
    pos: Pos2,
    cr: (u32, u32, u32, u32),
    rect: Rect,
    img_size: Vec2,
) -> Option<CropHandle> {
    use CropHandle::*;
    let mut best: Option<(f32, CropHandle)> = None;
    for h in [NW, N, NE, W, E, SW, S, SE] {
        let hp = crop_handle_screen_pos(cr, h, rect, img_size);
        let d = pos.distance(hp);
        if d <= HANDLE_GRAB_RADIUS && best.map_or(true, |(bd, _)| d < bd) {
            best = Some((d, h));
        }
    }
    best.map(|(_, h)| h)
}

fn update_crop_for_handle(
    cr: &mut (u32, u32, u32, u32),
    handle: CropHandle,
    ix: f32,
    iy: f32,
    img_w: u32,
    img_h: u32,
) {
    let right = cr.0 + cr.2;
    let bottom = cr.1 + cr.3;
    let px = ix.clamp(0.0, img_w as f32) as u32;
    let py = iy.clamp(0.0, img_h as f32) as u32;
    match handle {
        CropHandle::NW => {
            let nx = px.min(right.saturating_sub(1));
            let ny = py.min(bottom.saturating_sub(1));
            *cr = (nx, ny, right - nx, bottom - ny);
        }
        CropHandle::N => {
            let ny = py.min(bottom.saturating_sub(1));
            *cr = (cr.0, ny, cr.2, bottom - ny);
        }
        CropHandle::NE => {
            let nr = px.max(cr.0 + 1).min(img_w);
            let ny = py.min(bottom.saturating_sub(1));
            *cr = (cr.0, ny, nr - cr.0, bottom - ny);
        }
        CropHandle::W => {
            let nx = px.min(right.saturating_sub(1));
            *cr = (nx, cr.1, right - nx, cr.3);
        }
        CropHandle::E => {
            let nr = px.max(cr.0 + 1).min(img_w);
            *cr = (cr.0, cr.1, nr - cr.0, cr.3);
        }
        CropHandle::SW => {
            let nx = px.min(right.saturating_sub(1));
            let nb = py.max(cr.1 + 1).min(img_h);
            *cr = (nx, cr.1, right - nx, nb - cr.1);
        }
        CropHandle::S => {
            let nb = py.max(cr.1 + 1).min(img_h);
            *cr = (cr.0, cr.1, cr.2, nb - cr.1);
        }
        CropHandle::SE => {
            let nr = px.max(cr.0 + 1).min(img_w);
            let nb = py.max(cr.1 + 1).min(img_h);
            *cr = (cr.0, cr.1, nr - cr.0, nb - cr.1);
        }
    }
}

fn draw_crop_handles(
    painter: &egui::Painter,
    cr: (u32, u32, u32, u32),
    rect: Rect,
    img_size: Vec2,
    active: Option<CropHandle>,
) {
    use CropHandle::*;
    for h in [NW, N, NE, W, E, SW, S, SE] {
        let center = crop_handle_screen_pos(cr, h, rect, img_size);
        let is_active = active == Some(h);
        let fill = if is_active {
            Color32::from_rgb(0x00, 0x78, 0xD4)
        } else {
            Color32::WHITE
        };
        let r = if matches!(h, NW | NE | SW | SE) {
            HANDLE_RADIUS + 1.0
        } else {
            HANDLE_RADIUS
        };
        painter.circle_filled(center, r + 1.0, Color32::from_black_alpha(100));
        painter.circle_filled(center, r, fill);
        painter.circle_stroke(center, r, Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xD4)));
    }
}

fn draw_rulers(
    painter: &egui::Painter,
    rect: Rect,
    img_size: Vec2,
    cr: (u32, u32, u32, u32),
    dark: bool,
) {
    let scale_x = rect.width() / img_size.x.max(1.0);
    let scale_y = rect.height() / img_size.y.max(1.0);

    let intervals: &[u32] = &[1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000];
    let interval = intervals
        .iter()
        .find(|&&c| c as f32 * scale_x >= 50.0)
        .copied()
        .unwrap_or(5000);

    let ruler_bg = if dark {
        Color32::from_rgba_unmultiplied(0x1C, 0x1C, 0x1E, 210)
    } else {
        Color32::from_rgba_unmultiplied(0xF0, 0xF0, 0xF0, 210)
    };
    let tick_color = if dark { Color32::from_gray(130) } else { Color32::from_gray(100) };
    let text_color = if dark { Color32::from_gray(170) } else { Color32::from_gray(60) };
    let mark_color = Color32::from_rgba_unmultiplied(0x00, 0x78, 0xD4, 160);
    let font = egui::FontId::monospace(9.0);

    // ── Horizontal ruler (top edge) ─────────────────────────
    let h_ruler = Rect::from_min_size(
        Pos2::new(rect.min.x, rect.min.y - RULER_WIDTH),
        Vec2::new(rect.width(), RULER_WIDTH),
    );
    painter.rect_filled(h_ruler, egui::CornerRadius::ZERO, ruler_bg);
    let mut px = 0u32;
    let end_x = img_size.x as u32;
    while px <= end_x {
        let sx = rect.min.x + px as f32 * scale_x;
        if sx >= h_ruler.min.x && sx <= h_ruler.max.x {
            painter.line_segment(
                [Pos2::new(sx, h_ruler.max.y - 7.0), Pos2::new(sx, h_ruler.max.y)],
                Stroke::new(1.0, tick_color),
            );
            painter.text(
                Pos2::new(sx + 2.0, h_ruler.min.y + 3.0),
                egui::Align2::LEFT_TOP,
                format!("{px}"),
                font.clone(),
                text_color,
            );
        }
        px += interval;
    }
    // Crop boundary markers on horizontal ruler.
    for edge_px in [cr.0, cr.0 + cr.2] {
        let sx = rect.min.x + edge_px as f32 * scale_x;
        painter.line_segment(
            [Pos2::new(sx, h_ruler.min.y), Pos2::new(sx, h_ruler.max.y)],
            Stroke::new(2.0, mark_color),
        );
    }

    // ── Vertical ruler (left edge) ──────────────────────────
    let v_ruler = Rect::from_min_size(
        Pos2::new(rect.min.x - RULER_WIDTH, rect.min.y),
        Vec2::new(RULER_WIDTH, rect.height()),
    );
    painter.rect_filled(v_ruler, egui::CornerRadius::ZERO, ruler_bg);
    let interval_y = intervals
        .iter()
        .find(|&&c| c as f32 * scale_y >= 50.0)
        .copied()
        .unwrap_or(5000);
    let mut py = 0u32;
    let end_y = img_size.y as u32;
    while py <= end_y {
        let sy = rect.min.y + py as f32 * scale_y;
        if sy >= v_ruler.min.y && sy <= v_ruler.max.y {
            painter.line_segment(
                [Pos2::new(v_ruler.max.x - 7.0, sy), Pos2::new(v_ruler.max.x, sy)],
                Stroke::new(1.0, tick_color),
            );
            painter.text(
                Pos2::new(v_ruler.min.x + 1.0, sy + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{py}"),
                font.clone(),
                text_color,
            );
        }
        py += interval_y;
    }
    for edge_py in [cr.1, cr.1 + cr.3] {
        let sy = rect.min.y + edge_py as f32 * scale_y;
        painter.line_segment(
            [Pos2::new(v_ruler.min.x, sy), Pos2::new(v_ruler.max.x, sy)],
            Stroke::new(2.0, mark_color),
        );
    }

    // Corner square where the two rulers meet.
    let corner = Rect::from_min_size(
        Pos2::new(rect.min.x - RULER_WIDTH, rect.min.y - RULER_WIDTH),
        Vec2::new(RULER_WIDTH, RULER_WIDTH),
    );
    painter.rect_filled(corner, egui::CornerRadius::ZERO, ruler_bg);
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
