// Background removal via U²-Net (u2netp small variant, ~4 MB).
//
// The ONNX model is loaded lazily on first use. On Windows we prefer the
// DirectML execution provider for GPU acceleration, falling back to CPU.
//
// Pipeline:
//   1. Downscale source to 320x320 RGB float tensor (flat NCHW layout).
//   2. Normalise with U²-Net's mean/std.
//   3. Run session → [1, 1, 320, 320] saliency mask.
//   4. Upsample mask bilinearly back to original resolution.
//   5. Multiply source alpha by mask → transparent background.

use std::path::PathBuf;

use anyhow::Result;
use image::RgbaImage;

#[cfg(feature = "ai")]
use once_cell::sync::OnceCell;
#[cfg(feature = "ai")]
use ort::session::Session;

pub const MODEL_FILE: &str = "u2netp.onnx";

#[cfg(feature = "ai")]
static SESSION: OnceCell<parking_lot::Mutex<Session>> = OnceCell::new();

pub fn model_path() -> PathBuf {
    // Prefer the installer-bundled copy next to the exe.
    let bundled = crate::assets_dir().join("models").join(MODEL_FILE);
    if bundled.exists() {
        return bundled;
    }
    // Fall back to a user-writable dir for runtime downloads.
    crate::user_models_dir().join(MODEL_FILE)
}

#[cfg(feature = "ai")]
fn get_session() -> Result<&'static parking_lot::Mutex<Session>> {
    SESSION.get_or_try_init(|| {
        let path = model_path();
        if !path.exists() {
            anyhow::bail!("background removal model not found at {}", path.display());
        }
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort session builder: {e}"))?
            .commit_from_file(&path)
            .map_err(|e| anyhow::anyhow!("load u2netp model: {e}"))?;
        Ok(parking_lot::Mutex::new(session))
    })
}

#[cfg(feature = "ai")]
#[allow(clippy::erasing_op, clippy::identity_op)]
pub fn remove_background(image: &RgbaImage) -> Result<RgbaImage> {
    use crate::ops::resize::{resize_rgba, Filter};
    use ort::value::{Shape, Tensor};

    const SIZE: usize = 320;
    let small = resize_rgba(image, SIZE as u32, SIZE as u32, Filter::Bilinear)?;

    // Build NCHW float32 buffer, normalised with U²-Net's statistics.
    let mean = [0.485_f32, 0.456, 0.406];
    let std = [0.229_f32, 0.224, 0.225];
    let mut flat = vec![0.0f32; 3 * SIZE * SIZE];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = small.get_pixel(x as u32, y as u32).0;
            let r = (px[0] as f32 / 255.0 - mean[0]) / std[0];
            let g = (px[1] as f32 / 255.0 - mean[1]) / std[1];
            let b = (px[2] as f32 / 255.0 - mean[2]) / std[2];
            flat[0 * SIZE * SIZE + y * SIZE + x] = r;
            flat[1 * SIZE * SIZE + y * SIZE + x] = g;
            flat[2 * SIZE * SIZE + y * SIZE + x] = b;
        }
    }

    let shape = Shape::new([1i64, 3, SIZE as i64, SIZE as i64]);
    let input = Tensor::from_array((shape, flat))
        .map_err(|e| anyhow::anyhow!("build input tensor: {e}"))?;

    let sess_mutex = get_session()?;
    let mut session = sess_mutex.lock();
    let input_name = session
        .inputs()
        .first()
        .map(|i| i.name().to_string())
        .ok_or_else(|| anyhow::anyhow!("model has no input"))?;

    let outputs = session
        .run(ort::inputs![input_name => input])
        .map_err(|e| anyhow::anyhow!("run session: {e}"))?;

    let (_, first_out) = outputs
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("model returned no outputs"))?;
    let (out_shape, mask_slice) = first_out
        .try_extract_tensor::<f32>()
        .map_err(|e| anyhow::anyhow!("extract mask tensor: {e}"))?;

    // Expected shape [1, 1, H, W]; collapse to (H, W).
    let dims: Vec<i64> = out_shape.iter().copied().collect();
    let (mh, mw) = match dims.as_slice() {
        [1, 1, h, w] => (*h as usize, *w as usize),
        [1, h, w] => (*h as usize, *w as usize),
        [h, w] => (*h as usize, *w as usize),
        other => anyhow::bail!("unexpected mask shape {other:?}"),
    };

    // Normalise to [0, 1] based on observed min/max, then upscale to source.
    let mut min_v = f32::MAX;
    let mut max_v = f32::MIN;
    for v in mask_slice.iter() {
        if *v < min_v {
            min_v = *v;
        }
        if *v > max_v {
            max_v = *v;
        }
    }
    let range = (max_v - min_v).max(1e-6);
    let mut mask_img = image::GrayImage::new(mw as u32, mh as u32);
    for y in 0..mh {
        for x in 0..mw {
            let v = mask_slice[y * mw + x];
            let n = ((v - min_v) / range).clamp(0.0, 1.0);
            mask_img.put_pixel(x as u32, y as u32, image::Luma([(n * 255.0) as u8]));
        }
    }

    let mask_rgba = image::DynamicImage::ImageLuma8(mask_img).to_rgba8();
    let mask_full = crate::ops::resize::resize_rgba(
        &mask_rgba,
        image.width(),
        image.height(),
        crate::ops::resize::Filter::Bilinear,
    )?;

    let mut out = image.clone();
    for (dst, m) in out.pixels_mut().zip(mask_full.pixels()) {
        let mv = m.0[0];
        dst.0[3] = ((dst.0[3] as u16 * mv as u16) / 255) as u8;
    }
    Ok(out)
}

#[cfg(not(feature = "ai"))]
pub fn remove_background(_image: &RgbaImage) -> Result<RgbaImage> {
    anyhow::bail!("yImage was built without the `ai` feature")
}
