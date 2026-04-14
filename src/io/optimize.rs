// Image optimisation: reduce on-disk size while preserving visual quality.
//
// PNG  → oxipng lossless optimisation.
// JPEG → re-encode at user-chosen quality via mozjpeg (when the feature is
//        enabled) or the image crate's encoder as a fallback.
// WebP → webp crate quality slider.
//
// Each entry point writes to `dest` and returns the new file size so the UI
// can display before/after numbers.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use image::RgbaImage;

#[derive(Clone, Debug)]
pub struct OptimizeOptions {
    pub jpeg_quality: u8,
    pub png_level: u8,
    pub webp_quality: u8,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            jpeg_quality: 85,
            png_level: 3,
            webp_quality: 85,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptKind {
    Png,
    Jpeg,
    Webp,
}

pub fn kind_from_path(path: &Path) -> Option<OptKind> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => Some(OptKind::Png),
        Some("jpg" | "jpeg") => Some(OptKind::Jpeg),
        Some("webp") => Some(OptKind::Webp),
        _ => None,
    }
}

pub fn optimize_to(
    image: &RgbaImage,
    dest: &Path,
    opts: &OptimizeOptions,
) -> Result<u64> {
    let kind = kind_from_path(dest).ok_or_else(|| anyhow!("unsupported output format"))?;
    match kind {
        OptKind::Png => optimize_png(image, dest, opts.png_level),
        OptKind::Jpeg => optimize_jpeg(image, dest, opts.jpeg_quality),
        OptKind::Webp => optimize_webp(image, dest, opts.webp_quality),
    }
}

fn optimize_png(image: &RgbaImage, dest: &Path, level: u8) -> Result<u64> {
    // Encode to PNG first using the image crate (fast path), then feed that
    // buffer through oxipng for lossless size reduction.
    let mut initial = Vec::with_capacity(1024 * 64);
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut initial);
        use image::ImageEncoder;
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .context("encode png")?;
    }

    let level = level.clamp(0, 6);
    let mut oxi_opts = oxipng::Options::from_preset(level);
    oxi_opts.strip = oxipng::StripChunks::Safe;
    oxi_opts.optimize_alpha = true;

    let optimised = oxipng::optimize_from_memory(&initial, &oxi_opts)
        .context("oxipng optimise")?;
    std::fs::write(dest, &optimised).context("write optimised png")?;
    Ok(optimised.len() as u64)
}

fn optimize_jpeg(image: &RgbaImage, dest: &Path, quality: u8) -> Result<u64> {
    let quality = quality.clamp(1, 100);

    #[cfg(feature = "mozjpeg-optimize")]
    let bytes = {
        let w = image.width() as usize;
        let h = image.height() as usize;
        let rgb = image::DynamicImage::ImageRgba8(image.clone()).to_rgb8();
        let raw = rgb.into_raw();
        // mozjpeg can panic on unusual build targets; catch and fall back.
        let encoded = std::panic::catch_unwind(|| -> anyhow::Result<Vec<u8>> {
            let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
            comp.set_size(w, h);
            comp.set_quality(quality as f32);
            comp.set_progressive_mode();
            let mut started = comp
                .start_compress(Vec::with_capacity(raw.len() / 4))
                .map_err(|e| anyhow!("mozjpeg start_compress: {e}"))?;
            started
                .write_scanlines(&raw)
                .map_err(|e| anyhow!("mozjpeg write_scanlines: {e}"))?;
            let out = started
                .finish()
                .map_err(|e| anyhow!("mozjpeg finish: {e}"))?;
            Ok(out)
        });
        match encoded {
            Ok(Ok(v)) => v,
            _ => fallback_jpeg(&image::DynamicImage::ImageRgba8(image.clone()), quality)?,
        }
    };

    #[cfg(not(feature = "mozjpeg-optimize"))]
    let bytes = fallback_jpeg(&image::DynamicImage::ImageRgba8(image.clone()), quality)?;

    std::fs::write(dest, &bytes).context("write jpeg")?;
    Ok(bytes.len() as u64)
}

fn fallback_jpeg(dyn_img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    dyn_img
        .to_rgb8()
        .write_with_encoder(encoder)
        .context("fallback jpeg encode")?;
    Ok(out)
}

fn optimize_webp(image: &RgbaImage, dest: &Path, quality: u8) -> Result<u64> {
    let quality = quality.clamp(1, 100) as f32;
    let encoder = webp::Encoder::from_rgba(image.as_raw(), image.width(), image.height());
    let mem = encoder.encode(quality);
    let bytes: &[u8] = &mem;
    std::fs::write(dest, bytes).context("write webp")?;
    Ok(bytes.len() as u64)
}

/// Helper: pick a sensible default output path from the input path.
pub fn default_out_path(src: &Path) -> PathBuf {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-optimized.{ext}"))
}
