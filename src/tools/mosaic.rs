// Mosaic/pixelate tool. Replaces each block_size × block_size region inside
// the target rect with the average colour of that block.

use image::RgbaImage;

#[derive(Clone, Debug)]
pub struct MosaicState {
    pub block_size: u32,
    pub rect: Option<(u32, u32, u32, u32)>, // x, y, w, h
}

impl Default for MosaicState {
    fn default() -> Self {
        Self {
            block_size: 16,
            rect: None,
        }
    }
}

pub fn apply_mosaic(image: &mut RgbaImage, rect: (u32, u32, u32, u32), block_size: u32) {
    let (rx, ry, rw, rh) = rect;
    if block_size == 0 {
        return;
    }
    let img_w = image.width();
    let img_h = image.height();
    let x0 = rx.min(img_w.saturating_sub(1));
    let y0 = ry.min(img_h.saturating_sub(1));
    let x1 = (rx + rw).min(img_w);
    let y1 = (ry + rh).min(img_h);

    let mut by = y0;
    while by < y1 {
        let bh = (by + block_size).min(y1) - by;
        let mut bx = x0;
        while bx < x1 {
            let bw = (bx + block_size).min(x1) - bx;
            let (r, g, b, a) = block_average(image, bx, by, bw, bh);
            for yy in by..by + bh {
                for xx in bx..bx + bw {
                    image.get_pixel_mut(xx, yy).0 = [r, g, b, a];
                }
            }
            bx += block_size;
        }
        by += block_size;
    }
}

fn block_average(image: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> (u8, u8, u8, u8) {
    let mut rs: u64 = 0;
    let mut gs: u64 = 0;
    let mut bs: u64 = 0;
    let mut as_: u64 = 0;
    let count = (w * h) as u64;
    if count == 0 {
        return (0, 0, 0, 0);
    }
    for yy in y..y + h {
        for xx in x..x + w {
            let p = image.get_pixel(xx, yy).0;
            rs += p[0] as u64;
            gs += p[1] as u64;
            bs += p[2] as u64;
            as_ += p[3] as u64;
        }
    }
    (
        (rs / count) as u8,
        (gs / count) as u8,
        (bs / count) as u8,
        (as_ / count) as u8,
    )
}
