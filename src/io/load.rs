// Decode an image from disk into an RGBA8 buffer. Everything downstream of
// the loader assumes RGBA8, which keeps the viewer texture upload path
// branchless and lets filters operate without per-pixel channel checks.

use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};

pub fn load_image(path: &Path) -> Result<RgbaImage> {
    let reader = image::io::Reader::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("guess format {}", path.display()))?;
    let dyn_img: DynamicImage = reader
        .decode()
        .with_context(|| format!("decode {}", path.display()))?;
    Ok(dyn_img.to_rgba8())
}
