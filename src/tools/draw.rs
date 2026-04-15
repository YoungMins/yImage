// Drawing tools.
//
// This module hosts three related tool states:
//
//   * `BrushState`  — freehand soft brush + eraser (streams mouse positions
//                     into `stroke()` which composites an anti-aliased disk).
//   * `TextState`   — text-insertion tool. The user clicks once on the canvas
//                     and the current `content` string is rasterised in place.
//   * `ShapeState`  — rectangle / ellipse / line / arrow insertion tool. The
//                     viewer drags out a bounding box and calls `commit()`.
//
// Rasterisation uses `imageproc` where possible so we don't need a full 2D
// vector library. All ops write straight to the active RgbaImage; the caller
// is responsible for snapshotting undo state first.

use image::{Rgba, RgbaImage};
use imageproc::drawing;
use imageproc::rect::Rect;

#[derive(Clone, Debug)]
pub struct BrushState {
    pub color: [u8; 4],
    pub radius: f32,
    pub hardness: f32,
    pub eraser: bool,
    pub last_pos: Option<(f32, f32)>,
}

impl Default for BrushState {
    fn default() -> Self {
        Self {
            color: [0, 0, 0, 255],
            radius: 8.0,
            hardness: 0.8,
            eraser: false,
            last_pos: None,
        }
    }
}

impl BrushState {
    pub fn begin(&mut self) {
        self.last_pos = None;
    }

    /// Composite a stroke segment from the previous position (if any) to `pos`.
    pub fn stroke(&mut self, image: &mut RgbaImage, pos: (f32, f32)) {
        if let Some(prev) = self.last_pos {
            let dx = pos.0 - prev.0;
            let dy = pos.1 - prev.1;
            let dist = (dx * dx + dy * dy).sqrt();
            // Stamp every 0.35 * radius to overlap nicely.
            let step = (self.radius * 0.35).max(1.0);
            let n = ((dist / step).ceil() as usize).max(1);
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let x = prev.0 + dx * t;
                let y = prev.1 + dy * t;
                stamp(
                    image,
                    x,
                    y,
                    self.radius,
                    self.hardness,
                    self.color,
                    self.eraser,
                );
            }
        } else {
            stamp(
                image,
                pos.0,
                pos.1,
                self.radius,
                self.hardness,
                self.color,
                self.eraser,
            );
        }
        self.last_pos = Some(pos);
    }

    pub fn end(&mut self) {
        self.last_pos = None;
    }
}

fn stamp(
    image: &mut RgbaImage,
    cx: f32,
    cy: f32,
    radius: f32,
    hardness: f32,
    color: [u8; 4],
    eraser: bool,
) {
    let w = image.width() as i32;
    let h = image.height() as i32;
    let r = radius.ceil() as i32;
    let x0 = ((cx as i32) - r).max(0);
    let y0 = ((cy as i32) - r).max(0);
    let x1 = ((cx as i32) + r).min(w - 1);
    let y1 = ((cy as i32) + r).min(h - 1);
    let r2 = radius * radius;
    let soft_edge = (1.0 - hardness).max(0.001);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d2 = dx * dx + dy * dy;
            if d2 > r2 {
                continue;
            }
            let d = d2.sqrt();
            let inner = radius * hardness;
            let alpha_mul = if d <= inner {
                1.0
            } else {
                (1.0 - (d - inner) / (radius * soft_edge)).clamp(0.0, 1.0)
            };
            if eraser {
                let dst = image.get_pixel_mut(x as u32, y as u32);
                let new_a = (dst.0[3] as f32 * (1.0 - alpha_mul)).round() as u8;
                dst.0[3] = new_a;
            } else {
                let src_a = (color[3] as f32 / 255.0) * alpha_mul;
                if src_a <= 0.0 {
                    continue;
                }
                let dst = image.get_pixel_mut(x as u32, y as u32);
                *dst = blend(*dst, color, src_a);
            }
        }
    }
}

fn blend(dst: Rgba<u8>, src: [u8; 4], src_a: f32) -> Rgba<u8> {
    let inv = 1.0 - src_a;
    let out = [
        ((src[0] as f32 * src_a) + (dst[0] as f32 * inv)).round() as u8,
        ((src[1] as f32 * src_a) + (dst[1] as f32 * inv)).round() as u8,
        ((src[2] as f32 * src_a) + (dst[2] as f32 * inv)).round() as u8,
        ((src_a * 255.0) + (dst[3] as f32 * inv)).round().min(255.0) as u8,
    ];
    Rgba(out)
}

// ---------------------------------------------------------------------------
// Text tool
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TextState {
    pub content: String,
    pub font_size: f32,
    pub color: [u8; 4],
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            content: String::new(),
            font_size: 48.0,
            color: [255, 255, 255, 255],
        }
    }
}

impl TextState {
    /// Rasterise `content` with the embedded CJK font onto `image` at the
    /// given image-space position (top-left). Returns the number of glyphs
    /// actually painted so callers can skip committing when zero.
    pub fn stamp(&self, image: &mut RgbaImage, x: f32, y: f32) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        use ab_glyph::PxScale;
        let Some(font) = text_font() else {
            return 0;
        };
        let scale = PxScale::from(self.font_size);
        let color = Rgba(self.color);

        let mut line_y = y;
        let line_advance = self.font_size * 1.2;
        let mut glyphs = 0;
        for line in self.content.split('\n') {
            drawing::draw_text_mut(image, color, x as i32, line_y as i32, scale, &font, line);
            glyphs += line.chars().count();
            line_y += line_advance;
        }
        glyphs
    }
}

fn text_font() -> Option<ab_glyph::FontRef<'static>> {
    // Reuse the same font bytes already embedded for the UI — saves ~16 MB
    // of duplicated data.
    ab_glyph::FontRef::try_from_slice(crate::ui::CJK_FONT_BYTES).ok()
}

// ---------------------------------------------------------------------------
// Shape tool
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Rect,
    RectFilled,
    Ellipse,
    EllipseFilled,
    Line,
    Arrow,
}

#[derive(Clone, Debug)]
pub struct ShapeState {
    pub kind: ShapeKind,
    pub color: [u8; 4],
    pub stroke: f32,
    pub drag_start: Option<(f32, f32)>,
}

impl Default for ShapeState {
    fn default() -> Self {
        Self {
            kind: ShapeKind::Rect,
            color: [255, 0, 0, 255],
            stroke: 4.0,
            drag_start: None,
        }
    }
}

impl ShapeState {
    /// Commit the shape spanning `start` → `end` onto `image`.
    pub fn commit(&self, image: &mut RgbaImage, start: (f32, f32), end: (f32, f32)) {
        let color = Rgba(self.color);
        match self.kind {
            ShapeKind::Rect | ShapeKind::RectFilled => {
                let x0 = start.0.min(end.0) as i32;
                let y0 = start.1.min(end.1) as i32;
                let w = (start.0 - end.0).abs() as u32;
                let h = (start.1 - end.1).abs() as u32;
                if w == 0 || h == 0 {
                    return;
                }
                let rect = Rect::at(x0, y0).of_size(w.max(1), h.max(1));
                if matches!(self.kind, ShapeKind::RectFilled) {
                    drawing::draw_filled_rect_mut(image, rect, color);
                } else {
                    let s = self.stroke.max(1.0) as i32;
                    for i in 0..s {
                        let r = Rect::at(x0 + i, y0 + i).of_size(
                            w.saturating_sub((i as u32) * 2).max(1),
                            h.saturating_sub((i as u32) * 2).max(1),
                        );
                        drawing::draw_hollow_rect_mut(image, r, color);
                    }
                }
            }
            ShapeKind::Ellipse | ShapeKind::EllipseFilled => {
                let cx = ((start.0 + end.0) / 2.0) as i32;
                let cy = ((start.1 + end.1) / 2.0) as i32;
                let rx = ((start.0 - end.0).abs() / 2.0).max(1.0) as i32;
                let ry = ((start.1 - end.1).abs() / 2.0).max(1.0) as i32;
                if matches!(self.kind, ShapeKind::EllipseFilled) {
                    drawing::draw_filled_ellipse_mut(image, (cx, cy), rx, ry, color);
                } else {
                    drawing::draw_hollow_ellipse_mut(image, (cx, cy), rx, ry, color);
                }
            }
            ShapeKind::Line => {
                let s = self.stroke.max(1.0);
                stamp_line(image, start, end, s, self.color);
            }
            ShapeKind::Arrow => {
                let s = self.stroke.max(1.0);
                stamp_line(image, start, end, s, self.color);
                // Arrow head: two short lines at 30° off the direction.
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                let ux = dx / len;
                let uy = dy / len;
                let head = (len * 0.25).clamp(8.0, 40.0);
                let (cos_a, sin_a) = (0.866_f32, 0.5_f32); // cos/sin 30°
                let lx = -ux * cos_a + uy * sin_a;
                let ly = -uy * cos_a - ux * sin_a;
                let rx = -ux * cos_a - uy * sin_a;
                let ry = -uy * cos_a + ux * sin_a;
                let p1 = (end.0 + lx * head, end.1 + ly * head);
                let p2 = (end.0 + rx * head, end.1 + ry * head);
                stamp_line(image, end, p1, s, self.color);
                stamp_line(image, end, p2, s, self.color);
            }
        }
    }
}

/// Rasterise a thick line by stamping filled disks along the segment — avoids
/// imageproc's AA-line thickness limitations.
fn stamp_line(image: &mut RgbaImage, a: (f32, f32), b: (f32, f32), thickness: f32, color: [u8; 4]) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let step = (thickness * 0.5).max(1.0);
    let n = ((dist / step).ceil() as usize).max(1);
    let r = (thickness * 0.5).max(0.5);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = a.0 + dx * t;
        let y = a.1 + dy * t;
        stamp(image, x, y, r, 1.0, color, false);
    }
}
