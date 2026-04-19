// Build a GIF from a sequence of images.
//
// Uses the `gif` crate directly so we can keep the dependency footprint small.
// Each frame is quantised with `color_quant::NeuQuant` to a 256-entry palette
// for good quality without pulling in gifski.

use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use color_quant::NeuQuant;
use gif::{Encoder, Frame, Repeat};
use image::RgbaImage;
use rayon::prelude::*;

use crate::io::load::load_image;
use crate::ops::resize::{resize_rgba, Filter};

#[derive(Clone, Debug)]
pub struct GifOptions {
    /// Milliseconds per frame.
    pub delay_ms: u16,
    /// Loop count; 0 = infinite.
    pub loop_count: u16,
    /// If set, all frames are resized to (w, h) before encoding.
    pub target_size: Option<(u32, u32)>,
}

impl Default for GifOptions {
    fn default() -> Self {
        Self {
            delay_ms: 100,
            loop_count: 0,
            target_size: None,
        }
    }
}

pub fn build_gif_from_paths(inputs: &[PathBuf], out_path: &Path, opts: &GifOptions) -> Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("no input frames");
    }
    let frames: Vec<RgbaImage> = inputs
        .iter()
        .map(|p| load_image(p).with_context(|| format!("load {}", p.display())))
        .collect::<Result<_>>()?;
    build_gif(&frames, out_path, opts)
}

pub fn build_gif(frames: &[RgbaImage], out_path: &Path, opts: &GifOptions) -> Result<()> {
    let (w, h) = match opts.target_size {
        Some((w, h)) => (w, h),
        None => {
            let first = &frames[0];
            (first.width(), first.height())
        }
    };

    let delay = (opts.delay_ms / 10).max(1);

    let processed: Vec<Result<Frame<'static>>> = frames
        .par_iter()
        .map(|frame| {
            let resized = if frame.width() != w || frame.height() != h {
                resize_rgba(frame, w, h, Filter::Lanczos3)?
            } else {
                frame.clone()
            };
            let mut f = quantise_frame(&resized);
            f.delay = delay;
            Ok(f)
        })
        .collect();

    let file = File::create(out_path).with_context(|| format!("create {}", out_path.display()))?;
    let mut encoder = Encoder::new(file, w as u16, h as u16, &[]).context("create gif encoder")?;
    encoder
        .set_repeat(match opts.loop_count {
            0 => Repeat::Infinite,
            n => Repeat::Finite(n),
        })
        .context("set repeat")?;

    for frame_result in processed {
        encoder
            .write_frame(&frame_result?)
            .context("write gif frame")?;
    }

    Ok(())
}

fn quantise_frame(image: &RgbaImage) -> Frame<'static> {
    // NeuQuant expects RGBA interleaved. Sampling factor 10 is a good speed/
    // quality tradeoff; 1 is highest quality.
    let nq = NeuQuant::new(10, 256, image.as_raw());
    let palette_rgb: Vec<u8> = nq.color_map_rgb();

    let mut indexed = Vec::with_capacity((image.width() * image.height()) as usize);
    for px in image.pixels() {
        let idx = nq.index_of(&px.0) as u8;
        indexed.push(idx);
    }

    Frame {
        width: image.width() as u16,
        height: image.height() as u16,
        buffer: indexed.into(),
        palette: Some(palette_rgb),
        ..Frame::default()
    }
}
