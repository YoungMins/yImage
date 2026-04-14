// Drawing tool. Brushes composite onto the active image in image-space.
//
// The caller streams mouse positions (in image pixel coords) into `stroke`;
// each call composites an anti-aliased disk around the point and interpolates
// between the previous and current position so fast cursor movements still
// produce a continuous line.

use image::{Rgba, RgbaImage};

#[derive(Clone, Debug)]
pub struct BrushState {
    pub color: [u8; 4],
    pub radius: f32,
    pub hardness: f32,
    pub last_pos: Option<(f32, f32)>,
}

impl Default for BrushState {
    fn default() -> Self {
        Self {
            color: [0, 0, 0, 255],
            radius: 8.0,
            hardness: 0.8,
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
                stamp(image, x, y, self.radius, self.hardness, self.color);
            }
        } else {
            stamp(image, pos.0, pos.1, self.radius, self.hardness, self.color);
        }
        self.last_pos = Some(pos);
    }

    pub fn end(&mut self) {
        self.last_pos = None;
    }
}

fn stamp(image: &mut RgbaImage, cx: f32, cy: f32, radius: f32, hardness: f32, color: [u8; 4]) {
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
            // Feathered falloff between (radius * hardness) and radius.
            let inner = radius * hardness;
            let alpha_mul = if d <= inner {
                1.0
            } else {
                (1.0 - (d - inner) / (radius * soft_edge)).clamp(0.0, 1.0)
            };
            let src_a = (color[3] as f32 / 255.0) * alpha_mul;
            if src_a <= 0.0 {
                continue;
            }
            let dst = image.get_pixel_mut(x as u32, y as u32);
            *dst = blend(*dst, color, src_a);
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
