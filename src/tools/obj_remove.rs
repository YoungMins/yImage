// Object removal (image inpainting) via a LaMa ONNX model.
//
// Input:
//   - `image`: the source RgbaImage.
//   - `mask`:  an 8-bit mask the user painted over the object to remove
//              (white = remove, black = keep). Same dimensions as `image`.
//
// Pipeline:
//   1. Resize image+mask to 512×512 (LaMa's training resolution).
//   2. Pack into flat NCHW float buffers normalised to [0, 1].
//   3. Run session → inpainted RGB output.
//   4. Resize result back to source resolution and composite the painted
//      region only, preserving the rest of the source pixels.

use std::path::PathBuf;

use anyhow::Result;
use image::{GrayImage, RgbaImage};

#[cfg(feature = "ai")]
use once_cell::sync::OnceCell;
#[cfg(feature = "ai")]
use ort::session::Session;

pub const MODEL_FILE: &str = "lama.onnx";

#[cfg(feature = "ai")]
static SESSION: OnceCell<parking_lot::Mutex<Session>> = OnceCell::new();

pub fn model_path() -> PathBuf {
    crate::assets_dir().join("models").join(MODEL_FILE)
}

#[cfg(feature = "ai")]
fn get_session() -> Result<&'static parking_lot::Mutex<Session>> {
    SESSION.get_or_try_init(|| {
        let path = model_path();
        if !path.exists() {
            anyhow::bail!("object removal model not found at {}", path.display());
        }
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort session builder: {e}"))?
            .commit_from_file(&path)
            .map_err(|e| anyhow::anyhow!("load lama model: {e}"))?;
        Ok(parking_lot::Mutex::new(session))
    })
}

#[cfg(feature = "ai")]
#[allow(clippy::erasing_op, clippy::identity_op)]
pub fn inpaint(image: &RgbaImage, mask: &GrayImage) -> Result<RgbaImage> {
    use crate::ops::resize::{resize_rgba, Filter};
    use ort::value::{Shape, Tensor};

    anyhow::ensure!(
        mask.width() == image.width() && mask.height() == image.height(),
        "mask must match image size"
    );

    const SIZE: usize = 512;
    let small_img = resize_rgba(image, SIZE as u32, SIZE as u32, Filter::Bilinear)?;
    let mask_small = image::imageops::resize(
        mask,
        SIZE as u32,
        SIZE as u32,
        image::imageops::FilterType::Triangle,
    );

    // Flat NCHW RGB tensor.
    let mut img_flat = vec![0.0f32; 3 * SIZE * SIZE];
    let mut mask_flat = vec![0.0f32; 1 * SIZE * SIZE];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = small_img.get_pixel(x as u32, y as u32).0;
            img_flat[0 * SIZE * SIZE + y * SIZE + x] = p[0] as f32 / 255.0;
            img_flat[1 * SIZE * SIZE + y * SIZE + x] = p[1] as f32 / 255.0;
            img_flat[2 * SIZE * SIZE + y * SIZE + x] = p[2] as f32 / 255.0;
            let m = mask_small.get_pixel(x as u32, y as u32).0[0];
            mask_flat[y * SIZE + x] = if m > 127 { 1.0 } else { 0.0 };
        }
    }

    let img_shape = Shape::new([1i64, 3, SIZE as i64, SIZE as i64]);
    let mask_shape = Shape::new([1i64, 1, SIZE as i64, SIZE as i64]);
    let img_value = Tensor::from_array((img_shape, img_flat))
        .map_err(|e| anyhow::anyhow!("build image tensor: {e}"))?;
    let mask_value = Tensor::from_array((mask_shape, mask_flat))
        .map_err(|e| anyhow::anyhow!("build mask tensor: {e}"))?;

    let sess_mutex = get_session()?;
    let mut session = sess_mutex.lock();

    let input_names: Vec<String> = session
        .inputs()
        .iter()
        .map(|i| i.name().to_string())
        .collect();
    anyhow::ensure!(
        input_names.len() >= 2,
        "lama model expects image+mask inputs (got {})",
        input_names.len()
    );

    let outputs = session
        .run(ort::inputs![
            input_names[0].clone() => img_value,
            input_names[1].clone() => mask_value,
        ])
        .map_err(|e| anyhow::anyhow!("run lama: {e}"))?;

    let (_, first_out) = outputs
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no outputs"))?;
    let (out_shape, out_slice) = first_out
        .try_extract_tensor::<f32>()
        .map_err(|e| anyhow::anyhow!("extract lama output: {e}"))?;

    let dims: Vec<i64> = out_shape.iter().copied().collect();
    let (oh, ow) = match dims.as_slice() {
        [1, 3, h, w] => (*h as usize, *w as usize),
        other => anyhow::bail!("unexpected lama output shape {other:?}"),
    };

    let mut result_small = RgbaImage::new(ow as u32, oh as u32);
    for y in 0..oh {
        for x in 0..ow {
            let r = (out_slice[0 * oh * ow + y * ow + x].clamp(0.0, 1.0) * 255.0) as u8;
            let g = (out_slice[1 * oh * ow + y * ow + x].clamp(0.0, 1.0) * 255.0) as u8;
            let b = (out_slice[2 * oh * ow + y * ow + x].clamp(0.0, 1.0) * 255.0) as u8;
            result_small.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, 255]));
        }
    }

    let result_full = crate::ops::resize::resize_rgba(
        &result_small,
        image.width(),
        image.height(),
        crate::ops::resize::Filter::Lanczos3,
    )?;

    // Composite: original pixels where mask==0, inpainted where mask==1.
    let mut out = image.clone();
    for y in 0..image.height() {
        for x in 0..image.width() {
            let m = mask.get_pixel(x, y).0[0];
            if m > 127 {
                let src = result_full.get_pixel(x, y).0;
                let orig_a = out.get_pixel(x, y).0[3];
                out.put_pixel(x, y, image::Rgba([src[0], src[1], src[2], orig_a]));
            }
        }
    }
    Ok(out)
}

#[cfg(not(feature = "ai"))]
pub fn inpaint(_image: &RgbaImage, _mask: &GrayImage) -> Result<RgbaImage> {
    anyhow::bail!("yImage was built without the `ai` feature")
}
