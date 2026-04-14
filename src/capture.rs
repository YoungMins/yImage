// Windows screen capture using the Windows Graphics Capture (WGC) API via the
// `windows-capture` crate. WGC is the fastest, DWM-aware path on Windows 10
// 1903+: zero-copy from the compositor, no black overlays on protected
// content, and properly HDR-aware.
//
// For the MVP we expose a blocking `capture_primary_screen()` that grabs a
// single frame from the primary monitor. Window-picker and region selection
// will be layered on top in follow-ups.

#![cfg(windows)]

use anyhow::{Context, Result};
use image::RgbaImage;
use parking_lot::Mutex;
use std::sync::Arc;

use windows_capture::{
    capture::{Context as CapContext, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

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
        // The buffer comes back as BGRA8; swizzle to RGBA8 while copying.
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
