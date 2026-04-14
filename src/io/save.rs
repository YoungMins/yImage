// Encode an RGBA8 buffer to disk. Format is chosen from the target path's
// extension; unknown extensions fall back to PNG.

use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageFormat, RgbaImage};

pub fn save_image(img: &RgbaImage, path: &Path) -> Result<()> {
    let format = format_from_path(path).unwrap_or(ImageFormat::Png);
    let dyn_img = image::DynamicImage::ImageRgba8(img.clone());
    match format {
        ImageFormat::Jpeg => {
            // JPEG has no alpha; collapse to RGB.
            dyn_img
                .to_rgb8()
                .save_with_format(path, ImageFormat::Jpeg)
                .with_context(|| format!("save jpeg {}", path.display()))?;
        }
        f => {
            dyn_img
                .save_with_format(path, f)
                .with_context(|| format!("save {} {}", format_name(f), path.display()))?;
        }
    }
    Ok(())
}

pub fn format_from_path(path: &Path) -> Option<ImageFormat> {
    ImageFormat::from_path(path).ok()
}

fn format_name(f: ImageFormat) -> &'static str {
    match f {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::WebP => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
        ImageFormat::Avif => "avif",
        _ => "image",
    }
}
