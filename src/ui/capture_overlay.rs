// Full-window overlays for the interactive screen-capture modes.
//
// This module implements two overlays that appear *on top of* the main
// yImage window while a capture is being set up:
//
//   1. A **countdown banner** used by `CaptureMode::ActiveWindow` and
//      `CaptureMode::AutoScroll`. After the user picks the mode from the
//      menu, yImage shows a centred banner counting down (3 → 2 → 1) so
//      the user has time to switch focus to whichever window they want
//      captured. When the countdown reaches 0 the actual capture fires.
//
//   2. A **region crop overlay** used by `CaptureMode::Region` and
//      `CaptureMode::FixedRegion`. yImage first takes a full fullscreen
//      screenshot on a background thread. When the screenshot lands on
//      the UI thread the overlay opens: the screenshot fills the whole
//      window (letter-boxed to preserve aspect ratio) and the user drags
//      a rectangle over it to pick the final crop. Enter / mouse-release
//      confirms, Esc cancels.
//
// The overlay is a single `egui::Area` with `Order::Foreground`, covering
// the entire viewport and painting semi-opaque chrome on top of the
// existing panels.

#![cfg(all(windows, feature = "capture"))]

use std::time::Instant;

use egui::{Align2, Color32, ColorImage, FontId, Pos2, Rect, Sense, Stroke, TextureOptions, Vec2};
use image::RgbaImage;

use crate::app::{BgMsg, YImageApp};
use crate::capture::CaptureMode;

/// State driving the region-selection overlay.
pub struct RegionCropState {
    /// Fullscreen screenshot captured before the overlay opened.
    pub image: RgbaImage,
    /// Lazy GPU texture upload of `image`.
    pub texture: Option<egui::TextureHandle>,
    /// Pointer position where the current drag started (in overlay-local
    /// screen coordinates, i.e. egui `Pos2`).
    pub drag_start: Option<Pos2>,
    /// Pointer position during the current drag.
    pub drag_current: Option<Pos2>,
    /// Whether the user has released the mouse (the rectangle is final).
    pub finalised: bool,
    /// `Region` or `FixedRegion` — affects whether the selection is saved
    /// back to `DialogState::fixed_region` on confirm.
    pub mode: CaptureMode,
    /// Pre-saved fixed region (image-space coords) to show on overlay open.
    pub preset: Option<(i32, i32, u32, u32)>,
    /// Reusable texture handle for the magnifier loupe.
    pub magnifier_tex: Option<egui::TextureHandle>,
}

impl RegionCropState {
    pub fn new(image: RgbaImage, mode: CaptureMode) -> Self {
        Self {
            image,
            texture: None,
            drag_start: None,
            drag_current: None,
            finalised: false,
            mode,
            preset: None,
            magnifier_tex: None,
        }
    }
}

/// State driving the countdown banner.
#[derive(Clone)]
pub struct CaptureCountdown {
    pub mode: CaptureMode,
    pub started: Instant,
    pub total_secs: u64,
}

impl CaptureCountdown {
    pub fn new(mode: CaptureMode, total_secs: u64) -> Self {
        Self {
            mode,
            started: Instant::now(),
            total_secs,
        }
    }

    /// Returns the whole seconds remaining, clamped to `[0, total_secs]`.
    pub fn remaining(&self) -> u64 {
        let elapsed = self.started.elapsed().as_secs();
        self.total_secs.saturating_sub(elapsed)
    }

    /// True once the countdown has reached zero.
    pub fn is_done(&self) -> bool {
        self.remaining() == 0
    }
}

/// Render whichever overlay (if any) is currently active and pump the
/// countdown timer. Call this once per frame near the end of the UI pass
/// (after all panels / dialogs) so it lands on top.
pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    tick_countdown(ctx, app);
    show_countdown(ctx, app);
    show_capture_busy(ctx, app);
    show_region_crop(ctx, app);
}

/// Modal "capturing…" banner that appears whenever a capture is in flight
/// (post-countdown for ActiveWindow / AutoScroll, or any spawn_capture_*).
/// Active window / scroll captures can take several seconds; without this
/// banner the UI looks frozen between the countdown ending and the image
/// landing.
fn show_capture_busy(ctx: &egui::Context, app: &mut YImageApp) {
    // Only show while a capture is actively running. We piggy-back on the
    // shared `progress` slot, but suppress the banner while the countdown
    // or the region-crop overlay is on screen — they own the foreground.
    if app.dialog.capture_countdown.is_some() || app.dialog.region_crop.is_some() {
        return;
    }
    let Some((label, _)) = app.progress.clone() else {
        return;
    };

    // Repaint so the spinner animates and the banner closes promptly.
    ctx.request_repaint_after(std::time::Duration::from_millis(33));

    let screen = ctx.screen_rect();
    let dim = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("capture-busy-dim"),
    ));
    dim.rect_filled(screen, 0.0, Color32::from_black_alpha(120));

    let banner = Rect::from_center_size(screen.center(), Vec2::new(360.0, 120.0));
    egui::Area::new("capture-busy".into())
        .order(egui::Order::Foreground)
        .fixed_pos(banner.min)
        .interactable(false)
        .show(ctx, |ui| {
            let painter = ui.painter().clone();
            painter.rect_filled(
                banner,
                egui::CornerRadius::same(12),
                Color32::from_rgb(0x1E, 0x20, 0x24),
            );
            painter.rect_stroke(
                banner,
                egui::CornerRadius::same(12),
                Stroke::new(1.0, super::theme::ACCENT),
                egui::StrokeKind::Middle,
            );
            // Indeterminate spinner — egui::Spinner needs allocate space so
            // we draw it via a centred Ui child.
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(banner)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            child.add_space(20.0);
            child.add(egui::Spinner::new().size(28.0).color(super::theme::ACCENT));
            child.add_space(10.0);
            child.label(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(Color32::WHITE),
            );
        });
}

/// Advance the countdown and, when it reaches 0, actually spawn the
/// capture. Requests a repaint every frame while active so the banner
/// visibly ticks down.
fn tick_countdown(ctx: &egui::Context, app: &mut YImageApp) {
    let Some(countdown) = app.dialog.capture_countdown.as_ref() else {
        return;
    };
    if countdown.is_done() {
        let mode = countdown.mode;
        app.dialog.capture_countdown = None;
        app.spawn_capture_immediate(mode);
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn show_countdown(ctx: &egui::Context, app: &mut YImageApp) {
    let Some(countdown) = app.dialog.capture_countdown.clone() else {
        return;
    };

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("capture-countdown-dim"),
    ));
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(140));

    let banner_size = Vec2::new(520.0, 180.0);
    let banner_rect = Rect::from_center_size(screen.center(), banner_size);

    egui::Area::new("capture-countdown".into())
        .order(egui::Order::Foreground)
        .fixed_pos(banner_rect.min)
        .interactable(true)
        .show(ctx, |ui| {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(banner_rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            child.painter().rect_filled(
                banner_rect,
                egui::CornerRadius::same(12),
                Color32::from_rgb(0x1E, 0x20, 0x24),
            );
            child.painter().rect_stroke(
                banner_rect,
                egui::CornerRadius::same(12),
                Stroke::new(1.0, super::theme::ACCENT),
                egui::StrokeKind::Middle,
            );

            child.add_space(16.0);
            child.label(
                egui::RichText::new(app.i18n.t("cap-countdown-title", &[]))
                    .size(16.0)
                    .color(Color32::WHITE),
            );
            child.add_space(6.0);
            child.label(
                egui::RichText::new(app.i18n.t(
                    "cap-countdown-body",
                    &[("secs", countdown.remaining().to_string())],
                ))
                .size(12.0)
                .color(Color32::from_gray(200)),
            );
            child.add_space(10.0);

            // Huge digit countdown in the centre.
            let rem = countdown.remaining();
            child.painter().text(
                banner_rect.center() + Vec2::new(0.0, 14.0),
                Align2::CENTER_CENTER,
                format!("{rem}"),
                FontId::proportional(64.0),
                super::theme::ACCENT,
            );

            child.add_space(40.0);
            child.horizontal(|ui| {
                ui.add_space((banner_size.x - 120.0) * 0.5);
                if ui
                    .button(app.i18n.t("cap-countdown-cancel", &[]))
                    .clicked()
                {
                    app.dialog.capture_countdown = None;
                }
            });
        });
}

fn show_region_crop(ctx: &egui::Context, app: &mut YImageApp) {
    if app.dialog.region_crop.is_none() {
        return;
    }

    let monitor = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or_else(|| {
            let img = &app.dialog.region_crop.as_ref().unwrap().image;
            Vec2::new(img.width() as f32, img.height() as f32)
        });

    // Spawn a borderless, fullscreen, always-on-top viewport so the user
    // drags a rectangle on the actual desktop screenshot — not on a tiny
    // letterboxed copy inside the yImage window.
    let viewport_id = egui::ViewportId::from_hash_of("yimage-capture-crop");
    let viewport_builder = egui::ViewportBuilder::default()
        .with_title("Capture Region")
        .with_position(egui::pos2(0.0, 0.0))
        .with_inner_size(monitor)
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);

    let mut cancel = false;
    let mut confirm: Option<(u32, u32, u32, u32)> = None;

    ctx.show_viewport_immediate(viewport_id, viewport_builder, |vp_ctx, _class| {
        if vp_ctx.input(|i| i.viewport().close_requested()) {
            cancel = true;
            return;
        }

        // Upload the screenshot lazily. Texture lives on the parent ctx so
        // the GPU resource stays alive even though we render in vp_ctx.
        {
            let state = app.dialog.region_crop.as_mut().unwrap();
            if state.texture.is_none() {
                let (w, h) =
                    (state.image.width() as usize, state.image.height() as usize);
                let pixels: Vec<Color32> = state
                    .image
                    .pixels()
                    .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                let color_img = ColorImage {
                    size: [w, h],
                    source_size: egui::vec2(w as f32, h as f32),
                    pixels,
                };
                state.texture = Some(vp_ctx.load_texture(
                    "yimage_capture_shot",
                    color_img,
                    TextureOptions::LINEAR,
                ));
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(vp_ctx, |ui| {
                let screen = vp_ctx.screen_rect();
                let state = app.dialog.region_crop.as_mut().unwrap();
                let tex = state.texture.as_ref().unwrap().clone();
                let img_w = state.image.width() as f32;
                let img_h = state.image.height() as f32;

                // Render the screenshot at 1:1 — the viewport is sized to the
                // monitor, so dragging is on the actual desktop pixels.
                let scale = (screen.width() / img_w).min(screen.height() / img_h);
                let shot_size = Vec2::new(img_w * scale, img_h * scale);
                let shot_rect = Rect::from_center_size(screen.center(), shot_size);

                // If a preset fixed region exists, pre-draw the selection so
                // the user sees the saved rectangle immediately.
                if state.drag_start.is_none() {
                    if let Some((px, py, pw, ph)) = state.preset.take() {
                        let x0 = shot_rect.min.x
                            + (px as f32 / img_w) * shot_rect.width();
                        let y0 = shot_rect.min.y
                            + (py as f32 / img_h) * shot_rect.height();
                        let x1 = shot_rect.min.x
                            + ((px + pw as i32) as f32 / img_w) * shot_rect.width();
                        let y1 = shot_rect.min.y
                            + ((py + ph as i32) as f32 / img_h) * shot_rect.height();
                        state.drag_start = Some(Pos2::new(x0, y0));
                        state.drag_current = Some(Pos2::new(x1, y1));
                        state.finalised = true;
                    }
                }

                let painter = ui.painter().clone();
                painter.image(
                    tex.id(),
                    shot_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );

                // Title + hint chrome at the top of the overlay.
                let hint = app.i18n.t("cap-region-hint", &[]);
                let title = app.i18n.t("cap-region-title", &[]);
                let chrome_bg = Rect::from_min_size(
                    Pos2::new(screen.min.x, screen.min.y),
                    Vec2::new(screen.width(), 64.0),
                );
                painter.rect_filled(chrome_bg, 0.0, Color32::from_black_alpha(180));
                painter.text(
                    Pos2::new(screen.center().x, screen.min.y + 16.0),
                    Align2::CENTER_TOP,
                    &title,
                    FontId::proportional(18.0),
                    Color32::WHITE,
                );
                painter.text(
                    Pos2::new(screen.center().x, screen.min.y + 40.0),
                    Align2::CENTER_TOP,
                    &hint,
                    FontId::proportional(12.0),
                    Color32::from_gray(200),
                );

                // Drag interaction across the whole viewport.
                let response = ui.interact(
                    shot_rect,
                    egui::Id::new("capture-region-interact"),
                    Sense::click_and_drag(),
                );

                if response.drag_started() {
                    // `drag_started` only fires after egui's drag threshold is
                    // crossed, so `interact_pointer_pos` is already offset
                    // from the actual press. Use `press_origin` to snap the
                    // start back to the exact pixel the user clicked on —
                    // that's what the magnifier was showing a moment earlier.
                    let press = vp_ctx
                        .input(|i| i.pointer.press_origin())
                        .or_else(|| response.interact_pointer_pos());
                    if let Some(p) = press {
                        state.drag_start = Some(p);
                        state.drag_current = response
                            .interact_pointer_pos()
                            .or(Some(p));
                        state.finalised = false;
                    }
                }
                if response.dragged() {
                    if let Some(p) = response.interact_pointer_pos() {
                        state.drag_current = Some(p);
                    }
                }
                if response.drag_stopped() {
                    if let Some(p) = response.interact_pointer_pos() {
                        state.drag_current = Some(p);
                    }
                    state.finalised = true;
                }

                if let (Some(a), Some(b)) = (state.drag_start, state.drag_current) {
                    let sel = Rect::from_two_pos(a, b).intersect(shot_rect);
                    let outside = Color32::from_black_alpha(if state.finalised {
                        160
                    } else {
                        120
                    });
                    let top = Rect::from_min_max(
                        shot_rect.min,
                        Pos2::new(shot_rect.max.x, sel.min.y),
                    );
                    let bottom = Rect::from_min_max(
                        Pos2::new(shot_rect.min.x, sel.max.y),
                        shot_rect.max,
                    );
                    let left = Rect::from_min_max(
                        Pos2::new(shot_rect.min.x, sel.min.y),
                        Pos2::new(sel.min.x, sel.max.y),
                    );
                    let right = Rect::from_min_max(
                        Pos2::new(sel.max.x, sel.min.y),
                        Pos2::new(shot_rect.max.x, sel.max.y),
                    );
                    for r in [top, bottom, left, right] {
                        if r.width() > 0.0 && r.height() > 0.0 {
                            painter.rect_filled(r, 0.0, outside);
                        }
                    }
                    painter.rect_stroke(
                        sel,
                        egui::CornerRadius::ZERO,
                        Stroke::new(2.0, super::theme::ACCENT),
                        egui::StrokeKind::Middle,
                    );
                    let sel_img_w =
                        ((sel.width() / shot_rect.width()) * img_w).round() as u32;
                    let sel_img_h =
                        ((sel.height() / shot_rect.height()) * img_h).round() as u32;
                    if sel_img_w > 0 && sel_img_h > 0 {
                        let text = format!("{sel_img_w} × {sel_img_h} px");
                        painter.text(
                            Pos2::new(sel.min.x, sel.min.y - 6.0),
                            Align2::LEFT_BOTTOM,
                            text,
                            FontId::proportional(12.0),
                            Color32::WHITE,
                        );
                    }
                }

                // Magnifier loupe next to the cursor while dragging.
                if !state.finalised {
                    if let Some(cursor) = vp_ctx.input(|i| i.pointer.hover_pos()) {
                        if shot_rect.contains(cursor) {
                            draw_magnifier(
                                &painter,
                                &state.image,
                                shot_rect,
                                screen,
                                cursor,
                                img_w,
                                img_h,
                                vp_ctx,
                                &mut state.magnifier_tex,
                            );
                        }
                    }
                }

                // Bottom-centre action buttons.
                let actions_y = screen.max.y - 56.0;
                let actions_rect = Rect::from_center_size(
                    Pos2::new(screen.center().x, actions_y),
                    Vec2::new(360.0, 40.0),
                );
                let mut btn_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(actions_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                btn_ui.add_space(24.0);
                let confirm_label = app.i18n.t("cap-region-confirm", &[]);
                let cancel_label = app.i18n.t("cap-region-cancel", &[]);
                let has_selection = state
                    .drag_start
                    .zip(state.drag_current)
                    .map(|(a, b)| (a - b).length_sq() > 4.0)
                    .unwrap_or(false);
                if btn_ui
                    .add_enabled(
                        has_selection,
                        egui::Button::new(
                            egui::RichText::new(&confirm_label).color(Color32::WHITE),
                        )
                        .min_size(Vec2::new(140.0, 32.0))
                        .fill(super::theme::ACCENT),
                    )
                    .clicked()
                {
                    if let (Some(a), Some(b)) = (state.drag_start, state.drag_current) {
                        confirm = Some(selection_to_image_rect(
                            shot_rect, img_w, img_h, a, b,
                        ));
                    }
                }
                btn_ui.add_space(12.0);
                if btn_ui
                    .button(egui::RichText::new(&cancel_label).size(13.0))
                    .clicked()
                {
                    cancel = true;
                }

                vp_ctx.input(|i| {
                    if i.key_pressed(egui::Key::Escape) {
                        cancel = true;
                    }
                    if i.key_pressed(egui::Key::Enter) {
                        if let (Some(a), Some(b)) = (state.drag_start, state.drag_current)
                        {
                            confirm = Some(selection_to_image_rect(
                                shot_rect, img_w, img_h, a, b,
                            ));
                        }
                    }
                });
            });
    });

    if cancel {
        app.dialog.region_crop = None;
        return;
    }

    if let Some((x, y, w, h)) = confirm {
        if w > 0 && h > 0 {
            let state = app.dialog.region_crop.take().unwrap();
            let mode = state.mode;
            let cropped = match crop_rgba(&state.image, x, y, w, h) {
                Ok(img) => img,
                Err(e) => {
                    let _ = app.tx.send(BgMsg::Error(format!("crop: {e:#}")));
                    return;
                }
            };

            // If this was a FixedRegion capture, remember the rectangle in
            // screen coordinates so the next invocation of the menu item
            // can skip the overlay entirely.
            if matches!(mode, CaptureMode::FixedRegion { .. }) {
                app.dialog.fixed_region = Some((x as i32, y as i32, w, h));
                let _ = app.tx.send(BgMsg::Info(app.i18n.t(
                    "cap-fixed-saved",
                    &[
                        ("w", w.to_string()),
                        ("h", h.to_string()),
                        ("x", x.to_string()),
                        ("y", y.to_string()),
                    ],
                )));
            }

            // Route the cropped image through the usual ImageLoaded
            // pathway so it becomes the active document, same as every
            // other capture mode.
            let path = std::env::temp_dir()
                .join(format!("yimage-capture-{}.png", crate::app::unix_millis()));
            if let Err(e) = crate::io::save::save_image(&cropped, &path) {
                let _ = app.tx.send(BgMsg::Error(format!("save capture: {e:#}")));
                return;
            }
            let _ = app.tx.send(BgMsg::ImageLoaded {
                path,
                image: cropped,
                new_tab: true,
            });
        } else {
            app.dialog.region_crop = None;
        }
    }
}

/// Convert two pointer positions on the overlay-rect into an image-space
/// (u32, u32, u32, u32) rectangle.
fn selection_to_image_rect(
    shot_rect: Rect,
    img_w: f32,
    img_h: f32,
    a: Pos2,
    b: Pos2,
) -> (u32, u32, u32, u32) {
    let sel = Rect::from_two_pos(a, b).intersect(shot_rect);
    let u0 = ((sel.min.x - shot_rect.min.x) / shot_rect.width()).clamp(0.0, 1.0);
    let v0 = ((sel.min.y - shot_rect.min.y) / shot_rect.height()).clamp(0.0, 1.0);
    let u1 = ((sel.max.x - shot_rect.min.x) / shot_rect.width()).clamp(0.0, 1.0);
    let v1 = ((sel.max.y - shot_rect.min.y) / shot_rect.height()).clamp(0.0, 1.0);
    let x = (u0 * img_w).round() as u32;
    let y = (v0 * img_h).round() as u32;
    let w = ((u1 - u0) * img_w).round() as u32;
    let h = ((v1 - v0) * img_h).round() as u32;
    (x, y, w, h)
}

/// Draw a magnifier loupe near the cursor showing a zoomed-in view of the
/// screenshot pixels. Helps the user align the selection to exact pixel edges.
#[allow(clippy::too_many_arguments)]
fn draw_magnifier(
    painter: &egui::Painter,
    image: &RgbaImage,
    shot_rect: Rect,
    screen: Rect,
    cursor: Pos2,
    img_w: f32,
    img_h: f32,
    vp_ctx: &egui::Context,
    mag_tex: &mut Option<egui::TextureHandle>,
) {
    const RADIUS: i32 = 8;
    const SIDE: usize = (RADIUS * 2 + 1) as usize;
    const ZOOM: f32 = 8.0;
    const LOUPE_PX: f32 = SIDE as f32 * ZOOM;
    const OFFSET: f32 = 24.0;

    let u = ((cursor.x - shot_rect.min.x) / shot_rect.width()).clamp(0.0, 1.0);
    let v = ((cursor.y - shot_rect.min.y) / shot_rect.height()).clamp(0.0, 1.0);
    let ix = (u * img_w) as i32;
    let iy = (v * img_h) as i32;

    let mut pixels = Vec::with_capacity(SIDE * SIDE);
    for dy in -RADIUS..=RADIUS {
        for dx in -RADIUS..=RADIUS {
            let px = (ix + dx).clamp(0, img_w as i32 - 1) as u32;
            let py = (iy + dy).clamp(0, img_h as i32 - 1) as u32;
            let p = image.get_pixel(px, py);
            pixels.push(Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]));
        }
    }

    let color_img = ColorImage {
        size: [SIDE, SIDE],
        source_size: egui::vec2(SIDE as f32, SIDE as f32),
        pixels,
    };
    match mag_tex {
        Some(t) => t.set(color_img, TextureOptions::NEAREST),
        None => {
            *mag_tex = Some(vp_ctx.load_texture(
                "capture-magnifier",
                color_img,
                TextureOptions::NEAREST,
            ));
        }
    }
    let tex_id = mag_tex.as_ref().unwrap().id();

    // Position the loupe to the bottom-right of the cursor; flip if it
    // would go off-screen.
    let mut lx = cursor.x + OFFSET;
    let mut ly = cursor.y + OFFSET;
    if lx + LOUPE_PX + 4.0 > screen.max.x {
        lx = cursor.x - OFFSET - LOUPE_PX;
    }
    if ly + LOUPE_PX + 24.0 > screen.max.y {
        ly = cursor.y - OFFSET - LOUPE_PX - 24.0;
    }
    let loupe_rect = Rect::from_min_size(Pos2::new(lx, ly), Vec2::splat(LOUPE_PX));

    // Background + image.
    painter.rect_filled(
        loupe_rect.expand(3.0),
        egui::CornerRadius::same(6),
        Color32::from_black_alpha(200),
    );
    painter.image(
        tex_id,
        loupe_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.rect_stroke(
        loupe_rect,
        egui::CornerRadius::same(6),
        Stroke::new(2.0, Color32::from_white_alpha(200)),
        egui::StrokeKind::Outside,
    );

    // Crosshair lines through the center pixel.
    let cx = loupe_rect.center().x;
    let cy = loupe_rect.center().y;
    let hair = Color32::from_rgba_unmultiplied(255, 80, 80, 180);
    painter.line_segment(
        [Pos2::new(cx, loupe_rect.min.y), Pos2::new(cx, loupe_rect.max.y)],
        Stroke::new(1.0, hair),
    );
    painter.line_segment(
        [Pos2::new(loupe_rect.min.x, cy), Pos2::new(loupe_rect.max.x, cy)],
        Stroke::new(1.0, hair),
    );

    // Coordinate label below the loupe.
    painter.text(
        Pos2::new(loupe_rect.center().x, loupe_rect.max.y + 6.0),
        Align2::CENTER_TOP,
        format!("{ix}, {iy}"),
        FontId::proportional(12.0),
        Color32::WHITE,
    );
}

fn crop_rgba(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> anyhow::Result<RgbaImage> {
    let sw = src.width();
    let sh = src.height();
    let x = x.min(sw.saturating_sub(1));
    let y = y.min(sh.saturating_sub(1));
    let w = w.min(sw.saturating_sub(x)).max(1);
    let h = h.min(sh.saturating_sub(y)).max(1);
    Ok(image::imageops::crop_imm(src, x, y, w, h).to_image())
}
