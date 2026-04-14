// Format conversion. Single files go through io::save; batch mode fans work
// out across a rayon thread pool and reports progress via a channel.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use rayon::prelude::*;

use crate::io::{load::load_image, save::save_image};

#[derive(Clone, Debug)]
pub struct ConvertOptions {
    /// Target extension, e.g. "png", "webp", "jpg".
    pub target_ext: String,
    /// Optional output directory. If None, files are written next to the source.
    pub out_dir: Option<PathBuf>,
}

pub fn convert_one(src: &Path, opts: &ConvertOptions) -> Result<PathBuf> {
    let img = load_image(src).with_context(|| format!("load {}", src.display()))?;
    let dest = build_dest(src, opts);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    save_image(&img, &dest).with_context(|| format!("save {}", dest.display()))?;
    Ok(dest)
}

pub fn convert_batch(
    sources: &[PathBuf],
    opts: &ConvertOptions,
    progress: Option<Sender<(usize, usize, PathBuf)>>,
) -> Vec<Result<PathBuf>> {
    let total = sources.len();
    sources
        .par_iter()
        .enumerate()
        .map(|(i, src)| {
            let r = convert_one(src, opts);
            if let Some(tx) = &progress {
                let _ = tx.send((i + 1, total, src.clone()));
            }
            r
        })
        .collect()
}

fn build_dest(src: &Path, opts: &ConvertOptions) -> PathBuf {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let file = format!("{stem}.{}", opts.target_ext);
    match &opts.out_dir {
        Some(d) => d.join(file),
        None => src.parent().unwrap_or_else(|| Path::new(".")).join(file),
    }
}
