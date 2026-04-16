// Windows screen capture.
//
// Uses the Windows Graphics Capture (WGC) API via the `windows-capture` crate
// for fullscreen / window captures (zero-copy from the compositor, DWM-aware,
// HDR-aware on Windows 11). Region captures go through WGC then crop.
//
// Five capture modes are exposed:
//
//   * `CaptureMode::Fullscreen` — primary monitor, single frame.
//   * `CaptureMode::ActiveWindow` — the foreground window at the moment the
//      hotkey fires.
//   * `CaptureMode::Region` — interactive rectangle: we fullscreen-capture the
//      primary monitor and return it so the UI thread can present a crop
//      overlay to let the user pick a rectangle.
//   * `CaptureMode::FixedRegion { x, y, w, h }` — rectangle pinned from a
//      previous session / user setting, cropped out of a fullscreen capture.
//   * `CaptureMode::AutoScroll { ... }` — captures a scrolling window by
//      repeatedly snapping, sending PgDn, and stitching the vertical deltas.
//
// Each blocking `capture_*` function can safely be called from a background
// rayon worker; the UI thread is never blocked.

#![cfg(windows)]

use anyhow::{Context, Result};
use image::{GenericImage, RgbaImage};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

use windows_capture::{
    capture::{Context as CapContext, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
    window::Window,
};

/// One of the user-facing capture modes. The UI surfaces the first four
/// directly; `AutoScroll` is triggered via the dedicated menu entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    Fullscreen,
    ActiveWindow,
    Region,
    FixedRegion { x: i32, y: i32, w: u32, h: u32 },
    AutoScroll,
}

struct Grabber {
    output: Arc<Mutex<Option<RgbaImage>>>,
}

impl GraphicsCaptureApiHandler for Grabber {
    type Flags = Arc<Mutex<Option<RgbaImage>>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: CapContext<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { output: ctx.flags })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut buf = frame.buffer()?;
        let width = buf.width();
        let height = buf.height();
        let data = buf.as_nopadding_buffer()?;
        let mut rgba = Vec::with_capacity(data.len());
        for chunk in data.chunks_exact(4) {
            rgba.push(chunk[2]);
            rgba.push(chunk[1]);
            rgba.push(chunk[0]);
            rgba.push(chunk[3]);
        }
        if let Some(img) = RgbaImage::from_raw(width, height, rgba) {
            *self.output.lock() = Some(img);
        }
        capture_control.stop();
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Capture the primary monitor in one frame.
pub fn capture_primary_screen() -> Result<RgbaImage> {
    let monitor = Monitor::primary().context("get primary monitor")?;
    let output: Arc<Mutex<Option<RgbaImage>>> = Arc::new(Mutex::new(None));

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        output.clone(),
    );

    Grabber::start(settings).map_err(|e| anyhow::anyhow!("start capture: {e}"))?;

    let frame = output.lock().take();
    frame.context("capture ended with no frame")
}

/// Capture the current foreground window.
pub fn capture_active_window() -> Result<RgbaImage> {
    let window = Window::foreground().context("get foreground window")?;
    let output: Arc<Mutex<Option<RgbaImage>>> = Arc::new(Mutex::new(None));

    let settings = Settings::new(
        window,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        output.clone(),
    );

    Grabber::start(settings).map_err(|e| anyhow::anyhow!("start window capture: {e}"))?;

    let frame = output.lock().take();
    frame.context("capture ended with no frame")
}

/// Capture a fixed rectangle out of the primary monitor.
pub fn capture_fixed_region(x: i32, y: i32, w: u32, h: u32) -> Result<RgbaImage> {
    let full = capture_primary_screen()?;
    let fw = full.width() as i32;
    let fh = full.height() as i32;
    let x0 = x.max(0).min(fw.saturating_sub(1));
    let y0 = y.max(0).min(fh.saturating_sub(1));
    let x1 = (x + w as i32).max(0).min(fw);
    let y1 = (y + h as i32).max(0).min(fh);
    let cw = (x1 - x0).max(1) as u32;
    let ch = (y1 - y0).max(1) as u32;
    let mut out = RgbaImage::new(cw, ch);
    out.copy_from(
        &*image::imageops::crop_imm(&full, x0 as u32, y0 as u32, cw, ch),
        0,
        0,
    )?;
    Ok(out)
}

/// Auto-scroll capture: repeatedly snap the active window, send `PgDn`, and
/// stitch the non-overlapping vertical delta. Works for any scroll container
/// where PgDn advances the visible area; the user can pick a different key via
/// `send_key_vk` in a future revision.
///
/// This is intentionally naive — it captures up to `max_iterations` frames with
/// `delay_ms` between them. Each new capture is diffed against the previous
/// using a simple per-row hash; rows that match the tail of the previous image
/// are discarded and the rest is appended downward.
pub fn capture_auto_scroll(max_iterations: usize, delay_ms: u64) -> Result<RgbaImage> {
    use std::thread::sleep;

    let first = capture_active_window()?;
    let mut stitched = first.clone();
    let mut prev = first;

    for _ in 0..max_iterations {
        send_pgdn();
        sleep(Duration::from_millis(delay_ms));
        let next = capture_active_window()?;
        if next.dimensions() != prev.dimensions() {
            break;
        }
        // Detect the static header (address bar, toolbar, etc.) that
        // appears identically at the top of consecutive captures.
        let header = find_static_header(&prev, &next);
        let overlap = find_vertical_overlap_skip(&stitched, &next, header);
        let content_h = next.height().saturating_sub(header);
        if overlap >= content_h {
            break;
        }
        append_below_skip(&mut stitched, &next, overlap, header);
        prev = next;
    }

    Ok(stitched)
}

fn send_pgdn() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VK_NEXT,
    };
    unsafe {
        let down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_NEXT,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_NEXT,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let inputs = [down, up];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn hash_row(img: &RgbaImage, y: u32, max_w: usize) -> u64 {
    let row = img
        .as_raw()
        .chunks_exact(img.width() as usize * 4)
        .nth(y as usize)
        .unwrap_or(&[]);
    let mut h: u64 = 1469598103934665603;
    for &b in row.iter().take(max_w * 4) {
        h = h.wrapping_mul(1099511628211);
        h ^= b as u64;
    }
    h
}

/// Detect how many rows at the top of two consecutive captures are identical
/// (static header: title bar, address bar, toolbar, etc.). Stops at the first
/// row that differs and caps at one-third of the image height.
fn find_static_header(a: &RgbaImage, b: &RgbaImage) -> u32 {
    let w = a.width().min(b.width()) as usize;
    let max_check = (a.height().min(b.height()) / 3) as usize;
    let mut header = 0u32;
    for y in 0..max_check {
        if hash_row(a, y as u32, w) == hash_row(b, y as u32, w) {
            header = y as u32 + 1;
        } else {
            break;
        }
    }
    header
}

/// Like `find_vertical_overlap` but skips the first `skip_top` rows of `next`
/// (the static header) when searching for overlap with the bottom of `stitched`.
fn find_vertical_overlap_skip(stitched: &RgbaImage, next: &RgbaImage, skip_top: u32) -> u32 {
    let w = stitched.width().min(next.width()) as usize;
    let sh = stitched.height() as usize;
    let content_h = next.height().saturating_sub(skip_top) as usize;
    let max_try = sh.min(content_h);

    for overlap in (1..=max_try).rev() {
        let mut matched = true;
        for i in 0..overlap {
            let s = hash_row(stitched, (sh - overlap + i) as u32, w);
            let n = hash_row(next, skip_top + i as u32, w);
            if s != n {
                matched = false;
                break;
            }
        }
        if matched {
            return overlap as u32;
        }
    }
    0
}

/// Append the non-overlapping, non-header portion of `next` below `stitched`.
fn append_below_skip(stitched: &mut RgbaImage, next: &RgbaImage, overlap: u32, skip_top: u32) {
    let w = stitched.width().max(next.width());
    let content_h = next.height().saturating_sub(skip_top);
    let extra = content_h.saturating_sub(overlap);
    if extra == 0 {
        return;
    }
    let new_h = stitched.height() + extra;
    let mut out = RgbaImage::new(w, new_h);
    let _ = out.copy_from(stitched, 0, 0);
    let src_y = skip_top + overlap;
    let tail = image::imageops::crop_imm(next, 0, src_y, next.width(), extra);
    let _ = out.copy_from(&*tail, 0, stitched.height());
    *stitched = out;
}
