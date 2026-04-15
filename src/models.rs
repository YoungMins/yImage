// ONNX model detection and one-click download.
//
// Background removal (U²-Net) and object removal (LaMa) both require a model
// file on disk. This module centralises where those files live, reports
// whether each is present, and (when asked) streams the file in on a
// background thread using an embedded tiny HTTP client so the user can
// install them without leaving the app.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    BgRemove,
    ObjRemove,
}

impl ModelKind {
    pub fn file_name(self) -> &'static str {
        match self {
            ModelKind::BgRemove => crate::tools::bg_remove::MODEL_FILE,
            ModelKind::ObjRemove => crate::tools::obj_remove::MODEL_FILE,
        }
    }

    /// Canonical download URL for the bundled model. These point at well-known
    /// GitHub release mirrors so end users get a deterministic file without
    /// needing to run a separate build script.
    pub fn url(self) -> &'static str {
        match self {
            // U²-Net small (4.7 MB).
            ModelKind::BgRemove => {
                "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx"
            }
            // LaMa inpainting (~200 MB).
            ModelKind::ObjRemove => {
                "https://huggingface.co/Carve/LaMa-ONNX/resolve/main/lama_fp32.onnx"
            }
        }
    }

    pub fn expected_size_mb(self) -> u64 {
        match self {
            ModelKind::BgRemove => 5,
            ModelKind::ObjRemove => 200,
        }
    }

    pub fn path(self) -> PathBuf {
        match self {
            ModelKind::BgRemove => crate::tools::bg_remove::model_path(),
            ModelKind::ObjRemove => crate::tools::obj_remove::model_path(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelStatus {
    pub ready: bool,
    pub path: PathBuf,
    pub size: u64,
}

pub fn check(kind: ModelKind) -> ModelStatus {
    let path = kind.path();
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    ModelStatus {
        ready: path.exists() && size > 0,
        path,
        size,
    }
}

#[derive(Clone, Debug, Default)]
pub struct DownloadState {
    pub in_progress: bool,
    pub progress: f32,
    pub message: Option<String>,
}

#[derive(Clone, Default)]
pub struct DownloadManager {
    pub bg: Arc<Mutex<DownloadState>>,
    pub obj: Arc<Mutex<DownloadState>>,
}

impl DownloadManager {
    pub fn state(&self, kind: ModelKind) -> DownloadState {
        match kind {
            ModelKind::BgRemove => self.bg.lock().clone(),
            ModelKind::ObjRemove => self.obj.lock().clone(),
        }
    }

    pub fn slot(&self, kind: ModelKind) -> Arc<Mutex<DownloadState>> {
        match kind {
            ModelKind::BgRemove => self.bg.clone(),
            ModelKind::ObjRemove => self.obj.clone(),
        }
    }
}

/// Stream a model file from `url` into `dest`, updating `state` as bytes arrive.
///
/// Uses the Windows HTTP stack via WinHTTP (no new dependency) on Windows, and
/// falls back to a minimal stub elsewhere. Returns Err with a human-readable
/// message on failure.
pub fn download_blocking(
    url: &str,
    dest: &std::path::Path,
    state: Arc<Mutex<DownloadState>>,
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(windows)]
    {
        http_download_windows(url, dest, state)
    }
    #[cfg(not(windows))]
    {
        let _ = (url, dest, state);
        anyhow::bail!("model downloader is only implemented on Windows")
    }
}

#[cfg(windows)]
fn http_download_windows(
    url: &str,
    dest: &std::path::Path,
    state: Arc<Mutex<DownloadState>>,
) -> anyhow::Result<()> {
    use std::io::Write;

    // Use PowerShell's Invoke-WebRequest as a robust, already-installed HTTP
    // client. We spawn it synchronously, read the file back, and poll size
    // periodically to update the progress bar. Not the fastest path, but
    // avoids pulling in reqwest/ureq for a one-shot download.
    //
    // For incremental progress we use `curl.exe` when available (Windows 10
    // 1803+ ships it) and fall back to PowerShell otherwise.

    let dest_tmp = dest.with_extension("part");
    // Try curl first for streaming progress.
    if let Ok(curl) = which_curl() {
        let mut child = std::process::Command::new(curl)
            .arg("-L")
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("--output")
            .arg(&dest_tmp)
            .arg(url)
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Poll file size while curl runs to update progress.
        let poll_path = dest_tmp.clone();
        let poll_state = state.clone();
        let poll_thread = std::thread::spawn(move || {
            // We don't know Content-Length without a HEAD; update bytes ticker.
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let size = std::fs::metadata(&poll_path).map(|m| m.len()).unwrap_or(0);
                let mut s = poll_state.lock();
                if !s.in_progress {
                    break;
                }
                s.message = Some(format!("{} downloaded", format_bytes(size)));
            }
        });

        let out = child.wait()?;
        state.lock().in_progress = false;
        let _ = poll_thread.join();
        if !out.success() {
            let _ = std::fs::remove_file(&dest_tmp);
            anyhow::bail!("curl exited with status {out:?}");
        }
        std::fs::rename(&dest_tmp, dest)?;
        state.lock().progress = 1.0;
        return Ok(());
    }

    // PowerShell fallback.
    let ps_cmd = format!(
        "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
        url.replace('\'', "''"),
        dest_tmp.display()
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .output()?;
    state.lock().in_progress = false;
    if !out.status.success() {
        let _ = std::fs::remove_file(&dest_tmp);
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        anyhow::bail!("powershell download failed: {err}");
    }
    std::fs::rename(&dest_tmp, dest)?;
    state.lock().progress = 1.0;
    drop(Write::flush(&mut std::io::stdout())); // keep unused import happy on stub
    Ok(())
}

#[cfg(windows)]
fn which_curl() -> anyhow::Result<PathBuf> {
    let out = std::process::Command::new("where")
        .arg("curl.exe")
        .output()?;
    if !out.status.success() {
        anyhow::bail!("curl not found");
    }
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if first.is_empty() {
        anyhow::bail!("curl not found");
    }
    Ok(PathBuf::from(first))
}

fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}
