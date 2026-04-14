// SIMD-accelerated resize via `fast_image_resize`.
//
// The image crate's built-in resize is fine for one-off use but slow for the
// live-preview slider in our Resize dialog. `fast_image_resize` uses SSE4/AVX2
// on x86 and NEON on aarch64 to deliver 4-10x throughput.

use anyhow::{Context, Result};
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeOptions, Resizer};
use image::RgbaImage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Bilinear,
    Lanczos3,
}

impl Filter {
    fn to_algorithm(self) -> fast_image_resize::ResizeAlg {
        use fast_image_resize::{FilterType, ResizeAlg};
        match self {
            Filter::Nearest => ResizeAlg::Nearest,
            Filter::Bilinear => ResizeAlg::Convolution(FilterType::Bilinear),
            Filter::Lanczos3 => ResizeAlg::Convolution(FilterType::Lanczos3),
        }
    }
}

pub fn resize_rgba(src: &RgbaImage, new_w: u32, new_h: u32, filter: Filter) -> Result<RgbaImage> {
    if new_w == 0 || new_h == 0 {
        anyhow::bail!("resize target has zero dimension");
    }
    let src_img = Image::from_vec_u8(
        src.width(),
        src.height(),
        src.as_raw().clone(),
        PixelType::U8x4,
    )
    .context("wrap source for resize")?;
    let mut dst_img = Image::new(new_w, new_h, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let opts = ResizeOptions::new().resize_alg(filter.to_algorithm());
    resizer
        .resize(&src_img, &mut dst_img, &opts)
        .context("resize")?;

    let buf = dst_img.into_vec();
    RgbaImage::from_raw(new_w, new_h, buf).context("build rgba from resized buffer")
}

/// Preserve aspect ratio: compute a height that matches the source ratio for
/// the given width (or vice-versa). Pass 0 for the axis you want computed.
pub fn aspect_fit(src_w: u32, src_h: u32, target_w: u32, target_h: u32) -> (u32, u32) {
    if target_w > 0 && target_h == 0 {
        let h = (src_h as f32 * (target_w as f32 / src_w as f32)).round() as u32;
        (target_w, h.max(1))
    } else if target_h > 0 && target_w == 0 {
        let w = (src_w as f32 * (target_h as f32 / src_h as f32)).round() as u32;
        (w.max(1), target_h)
    } else {
        (target_w.max(1), target_h.max(1))
    }
}
