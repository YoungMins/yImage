// Top-level application state. Holds the current document, UI state, tool
// selection, i18n bundle, settings, and the channel used by background workers
// to push finished work back to the UI thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use parking_lot::Mutex;

use crate::document::Document;
use crate::i18n::I18n;
use crate::io::load::load_image;
use crate::models::{DownloadManager, ModelKind};
use crate::tools::ToolKind;
use crate::ui;

#[cfg(all(windows, feature = "capture"))]
use crate::hotkeys::{HotkeyAction, HotkeyRegistry};

// ── Tab ─────────────────────────────────────────────────────────────

/// A single editor tab: document + per-tab view/texture state.
pub struct Tab {
    pub id: usize,
    pub doc: Document,
    pub texture: Option<egui::TextureHandle>,
    pub texture_dirty: bool,
    pub viewer: ui::viewer::ViewerState,
}

impl Tab {
    pub fn new(id: usize, doc: Document) -> Self {
        Self {
            id,
            doc,
            texture: None,
            texture_dirty: true,
            viewer: ui::viewer::ViewerState::default(),
        }
    }

    pub fn title(&self) -> String {
        self.doc
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|f| f.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }
}

// ── Messages ────────────────────────────────────────────────────────

/// Messages background workers can post to the UI thread.
#[derive(Debug)]
pub enum BgMsg {
    ImageLoaded {
        path: PathBuf,
        image: image::RgbaImage,
        new_tab: bool,
    },
    ImageSaved(PathBuf),
    Progress(String, f32),
    Error(String),
    Info(String),
    /// A full-screen screenshot was captured and should be handed to the
    /// region-crop overlay (for `Region` / `FixedRegion` capture modes)
    /// instead of loaded as a document directly.
    #[cfg(all(windows, feature = "capture"))]
    CaptureScreenshot {
        image: image::RgbaImage,
        mode: crate::capture::CaptureMode,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StartupAction {
    #[default]
    Open,
    Optimize,
    Resize,
    Convert,
    BackgroundRemove,
    ObjectRemove,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Settings {
    pub language: String,
    pub theme_dark: bool,
    pub last_folder: Option<PathBuf>,
    pub jpeg_quality: u8,
    pub png_level: u8,
    pub webp_quality: u8,
    #[serde(default)]
    pub thumbs_visible: bool,
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    /// Global hotkey bindings (action key → HotKey spec string). Persists
    /// across restarts so users only need to set them once.
    #[serde(default, with = "hotkey_map_serde")]
    pub hotkeys: HotkeyConfig,
}

#[cfg(all(windows, feature = "capture"))]
pub type HotkeyConfig = HashMap<HotkeyAction, String>;

#[cfg(not(all(windows, feature = "capture")))]
pub type HotkeyConfig = HashMap<String, String>;

// Since HotkeyAction is an enum (only exists on windows+capture), serde
// it as a string-keyed map on the wire. On non-windows the raw map is
// serialised directly.
mod hotkey_map_serde {
    use super::HotkeyConfig;
    use serde::{Deserializer, Serializer};

    #[cfg(all(windows, feature = "capture"))]
    pub fn serialize<S: Serializer>(value: &HotkeyConfig, ser: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = ser.serialize_map(Some(value.len()))?;
        for (k, v) in value {
            m.serialize_entry(k.as_key(), v)?;
        }
        m.end()
    }

    #[cfg(all(windows, feature = "capture"))]
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<HotkeyConfig, D::Error> {
        use crate::hotkeys::HotkeyAction;
        use serde::Deserialize;
        use std::collections::HashMap as Map;
        let raw: Map<String, String> = Map::deserialize(de)?;
        let mut out = HotkeyConfig::new();
        for (k, v) in raw {
            let action = match k.as_str() {
                "capture.fullscreen" => HotkeyAction::CaptureFullscreen,
                "capture.window" => HotkeyAction::CaptureActiveWindow,
                "capture.region" => HotkeyAction::CaptureRegion,
                "capture.fixed" => HotkeyAction::CaptureFixedRegion,
                "capture.scroll" => HotkeyAction::CaptureAutoScroll,
                _ => continue,
            };
            out.insert(action, v);
        }
        Ok(out)
    }

    #[cfg(not(all(windows, feature = "capture")))]
    pub fn serialize<S: Serializer>(value: &HotkeyConfig, ser: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(value, ser)
    }

    #[cfg(not(all(windows, feature = "capture")))]
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<HotkeyConfig, D::Error> {
        serde::Deserialize::deserialize(de)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: crate::i18n::detect_locale(),
            theme_dark: true,
            last_folder: None,
            jpeg_quality: 85,
            png_level: 3,
            webp_quality: 85,
            thumbs_visible: false,
            recent_files: Vec::new(),
            #[cfg(all(windows, feature = "capture"))]
            hotkeys: crate::hotkeys::defaults(),
            #[cfg(not(all(windows, feature = "capture")))]
            hotkeys: HashMap::new(),
        }
    }
}

pub struct YImageApp {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    next_tab_id: usize,
    pub tool: ToolKind,
    pub i18n: I18n,
    pub settings: Settings,
    pub status: String,
    pub progress: Option<(String, f32)>,
    pub tx: Sender<BgMsg>,
    pub rx: Receiver<BgMsg>,
    pub dialog: ui::dialogs::DialogState,
    pub folder_entries: Arc<Mutex<Vec<PathBuf>>>,
    pub folder_index: usize,
    pub pending_action: StartupAction,
    pub thumbs: ui::thumbnails::Thumbnails,
    pub downloads: DownloadManager,
    #[cfg(all(windows, feature = "capture"))]
    pub hotkeys: Option<HotkeyRegistry>,
}

impl YImageApp {
    pub fn new(
        cc: &CreationContext<'_>,
        startup_file: Option<PathBuf>,
        startup_action: StartupAction,
    ) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, "settings"))
            .unwrap_or_default();

        let i18n = I18n::new(&settings.language);
        // Re-install the full style with the user's saved theme preference.
        // `main.rs` did a first install to make sure the first frame renders
        // in dark mode; redo it here with the correct theme.
        ui::theme::install(&cc.egui_ctx, settings.theme_dark);

        let (tx, rx) = crossbeam_channel::unbounded();

        let mut thumbs = ui::thumbnails::Thumbnails::new();
        thumbs.visible = settings.thumbs_visible;

        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 0,
            tool: ToolKind::None,
            i18n,
            settings,
            status: String::new(),
            progress: None,
            tx,
            rx,
            dialog: ui::dialogs::DialogState::default(),
            folder_entries: Arc::new(Mutex::new(Vec::new())),
            folder_index: 0,
            pending_action: startup_action,
            thumbs,
            downloads: DownloadManager::default(),
            #[cfg(all(windows, feature = "capture"))]
            hotkeys: None,
        };

        #[cfg(all(windows, feature = "capture"))]
        {
            match HotkeyRegistry::new() {
                Ok(mut reg) => {
                    reg.apply(&app.settings.hotkeys);
                    app.hotkeys = Some(reg);
                }
                Err(e) => {
                    tracing::warn!("failed to init hotkey registry: {e}");
                }
            }
        }

        if let Some(path) = startup_file {
            app.open_path(&path, true);
        }

        app
    }

    // ── Tab helpers ─────────────────────────────────────────────────

    pub fn has_doc(&self) -> bool {
        !self.tabs.is_empty()
    }

    pub fn active_doc(&self) -> Option<&Document> {
        self.tabs.get(self.active_tab).map(|t| &t.doc)
    }

    pub fn active_doc_mut(&mut self) -> Option<&mut Document> {
        self.tabs.get_mut(self.active_tab).map(|t| &mut t.doc)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn push_recent(&mut self, path: &Path) {
        const MAX_RECENT: usize = 8;
        self.settings.recent_files.retain(|p| p != path);
        self.settings.recent_files.insert(0, path.to_path_buf());
        self.settings.recent_files.truncate(MAX_RECENT);
    }

    pub fn set_texture_dirty(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.texture_dirty = true;
        }
    }

    fn add_tab(&mut self, doc: Document) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::new(id, doc));
        self.active_tab = self.tabs.len() - 1;
        self.dialog.obj_mask = None;
        self.dialog.obj_mask_tex = None;
    }

    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            // If we closed a tab before the active one, shift the index down.
            if self.active_tab > index && self.active_tab > 0 {
                self.active_tab -= 1;
            }
            // Clamp in case the active tab was the last one.
            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Save the current document to its known path. If no path exists, fall
    /// back to the "Save As" dialog.
    pub fn save_current(&mut self) {
        let idx = self.active_tab;
        let Some(tab) = self.tabs.get(idx) else {
            return;
        };
        let Some(path) = tab.doc.path.clone() else {
            self.dialog.save_dialog_open = true;
            return;
        };
        let image = tab.doc.image.clone();
        let tx = self.tx.clone();
        rayon::spawn(move || {
            if let Err(e) = crate::io::save::save_image(&image, &path) {
                let _ = tx.send(BgMsg::Error(format!("{e:#}")));
            } else {
                let _ = tx.send(BgMsg::ImageSaved(path));
            }
        });
        if let Some(tab) = self.tabs.get_mut(idx) {
            tab.doc.dirty = false;
        }
    }

    fn apply_pending_action(&mut self) {
        let action = std::mem::replace(&mut self.pending_action, StartupAction::Open);
        match action {
            StartupAction::Open => {}
            StartupAction::Optimize => {
                self.dialog.optimize_open = true;
            }
            StartupAction::Resize => {
                self.dialog.resize_open = true;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    self.dialog.resize_w = tab.doc.width();
                    self.dialog.resize_h = tab.doc.height();
                }
            }
            StartupAction::Convert => {
                self.dialog.convert_open = true;
            }
            StartupAction::BackgroundRemove => {
                self.tool = ToolKind::BackgroundRemove;
                self.run_background_remove();
            }
            StartupAction::ObjectRemove => {
                self.tool = ToolKind::ObjectRemove;
                self.status = self.i18n.t("obj-remove-hint", &[]);
            }
        }
    }

    pub fn run_background_remove(&mut self) {
        let Some(doc) = self.active_doc() else { return };
        let image = doc.image.clone();
        let tx = self.tx.clone();
        let label = self.i18n.t("tool-bg-remove", &[]);
        let _ = tx.send(BgMsg::Progress(label.clone(), 0.1));
        rayon::spawn(move || {
            #[cfg(feature = "ai")]
            match crate::tools::bg_remove::remove_background(&image) {
                Ok(out) => {
                    let _ = tx.send(BgMsg::ImageLoaded {
                        path: PathBuf::new(),
                        image: out,
                        new_tab: false,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
            #[cfg(not(feature = "ai"))]
            {
                let _ = (image, label);
                let _ = tx.send(BgMsg::Error("built without the `ai` feature".to_string()));
            }
        });
    }

    pub fn run_object_remove(&mut self) {
        let Some(doc) = self.active_doc() else { return };
        let Some(mask) = self.dialog.obj_mask.clone() else {
            return;
        };
        let image = doc.image.clone();
        let tx = self.tx.clone();
        let label = self.i18n.t("tool-obj-remove", &[]);
        let _ = tx.send(BgMsg::Progress(label.clone(), 0.1));
        rayon::spawn(move || {
            #[cfg(feature = "ai")]
            match crate::tools::obj_remove::inpaint(&image, &mask) {
                Ok(out) => {
                    let _ = tx.send(BgMsg::ImageLoaded {
                        path: PathBuf::new(),
                        image: out,
                        new_tab: false,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
            #[cfg(not(feature = "ai"))]
            {
                let _ = (image, mask, label);
                let _ = tx.send(BgMsg::Error("built without the `ai` feature".to_string()));
            }
        });
    }

    pub fn download_state(&self, kind: ModelKind) -> crate::models::DownloadState {
        self.downloads.state(kind)
    }

    pub fn download_model(&self, kind: ModelKind) {
        let slot = self.downloads.slot(kind);
        {
            let mut s = slot.lock();
            if s.in_progress {
                return;
            }
            s.in_progress = true;
            s.progress = 0.0;
            s.message = None;
        }
        let tx = self.tx.clone();
        let url = kind.url().to_string();
        let dest = kind.path();
        rayon::spawn(move || {
            let _ = tx.send(BgMsg::Info(format!("downloading {}", url)));
            match crate::models::download_blocking(&url, &dest, slot.clone()) {
                Ok(()) => {
                    let _ = tx.send(BgMsg::Info(format!("downloaded {}", dest.display())));
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::Error(format!("download: {e:#}")));
                }
            }
            slot.lock().in_progress = false;
        });
    }

    pub fn open_path(&mut self, path: &Path, new_tab: bool) {
        let tx = self.tx.clone();
        let path = path.to_path_buf();
        self.settings.last_folder = path.parent().map(Path::to_path_buf);
        self.scan_folder(&path);
        rayon::spawn(move || match load_image(&path) {
            Ok(img) => {
                let _ = tx.send(BgMsg::ImageLoaded {
                    path,
                    image: img,
                    new_tab,
                });
            }
            Err(e) => {
                let _ = tx.send(BgMsg::Error(format!("{e:#}")));
            }
        });
    }

    fn scan_folder(&self, current: &Path) {
        self.scan_folder_now(current);
    }

    pub fn scan_folder_now(&self, current: &Path) {
        let Some(dir) = current.parent() else { return };
        let dir = dir.to_path_buf();
        let entries = self.folder_entries.clone();
        rayon::spawn(move || {
            let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
                Ok(rd) => rd
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| is_supported_image(p))
                    .collect(),
                Err(_) => return,
            };
            files.sort();
            *entries.lock() = files;
        });
    }

    pub fn navigate(&mut self, delta: isize) {
        let entries = self.folder_entries.lock().clone();
        if entries.is_empty() {
            return;
        }
        let current = self
            .active_doc()
            .and_then(|d| d.path.as_ref())
            .and_then(|p| entries.iter().position(|e| e == p))
            .unwrap_or(self.folder_index);
        let len = entries.len() as isize;
        let next = ((current as isize + delta).rem_euclid(len)) as usize;
        self.folder_index = next;
        let path = entries[next].clone();
        let is_dirty = self.tabs.get(self.active_tab).map_or(false, |t| t.doc.dirty);
        self.open_path(&path, is_dirty);
    }

    /// High-level menu/hotkey entry point for all capture modes.
    ///
    /// Routes each mode to the right UX:
    ///
    ///   * `Fullscreen` — capture immediately, open as a new document.
    ///   * `ActiveWindow` / `AutoScroll` — start a countdown so the user
    ///     has time to bring the target window to the foreground. The
    ///     actual capture fires from the countdown tick in
    ///     `ui::capture_overlay`.
    ///   * `Region` — capture a fullscreen screenshot in the background
    ///     and, when it arrives on the UI thread, open the region-crop
    ///     overlay so the user can drag the final rectangle.
    ///   * `FixedRegion` — same overlay flow, except the selection is
    ///     saved to `dialog.fixed_region` for reuse. If we already have
    ///     a saved region from a previous session we skip the overlay.
    #[cfg(all(windows, feature = "capture"))]
    pub fn trigger_capture(&mut self, mode: crate::capture::CaptureMode) {
        use crate::capture::CaptureMode;
        match mode {
            CaptureMode::Fullscreen => {
                self.spawn_capture_immediate(mode);
            }
            CaptureMode::ActiveWindow | CaptureMode::AutoScroll => {
                self.dialog.capture_countdown =
                    Some(crate::ui::capture_overlay::CaptureCountdown::new(mode, 3));
            }
            CaptureMode::Region => {
                self.spawn_capture_screenshot_for_crop(mode);
            }
            CaptureMode::FixedRegion { .. } => {
                // Always open the overlay. For a fresh first-time capture
                // the user draws the rectangle; on subsequent captures the
                // previously saved rectangle is pre-drawn and the user
                // confirms it (Enter / Capture button) or adjusts it by
                // dragging a new one before the capture actually fires.
                self.spawn_capture_screenshot_for_crop(CaptureMode::FixedRegion {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                });
            }
        }
    }

    /// Background-thread capture that feeds the result straight into the
    /// document pipeline. Used by modes that don't need the region-crop
    /// overlay (Fullscreen / pre-saved FixedRegion / post-countdown
    /// ActiveWindow / AutoScroll).
    ///
    /// For modes that capture the full monitor (Fullscreen, Region,
    /// FixedRegion) the yImage window is minimized first so it doesn't
    /// appear in the screenshot. The window is restored after the capture.
    #[cfg(all(windows, feature = "capture"))]
    pub fn spawn_capture_immediate(&mut self, mode: crate::capture::CaptureMode) {
        use crate::capture::CaptureMode;
        self.status = self.i18n.t("status-capturing", &[]);
        self.progress = Some((self.i18n.t("status-capturing", &[]), 0.0));
        let tx = self.tx.clone();

        // Grab our window handle while on the UI thread so we can
        // minimize/restore from the background thread.
        let needs_hide = matches!(
            mode,
            CaptureMode::Fullscreen | CaptureMode::Region | CaptureMode::FixedRegion { .. }
        );
        let hwnd = if needs_hide { self_hwnd() } else { None };

        std::thread::spawn(move || {
            if let Some(h) = hwnd {
                minimize_window(h);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }

            let result = match mode {
                CaptureMode::Fullscreen => crate::capture::capture_primary_screen(),
                CaptureMode::ActiveWindow => crate::capture::capture_active_window(),
                CaptureMode::Region => crate::capture::capture_primary_screen(),
                CaptureMode::FixedRegion { x, y, w, h } => {
                    crate::capture::capture_fixed_region(x, y, w, h)
                }
                CaptureMode::AutoScroll => crate::capture::capture_auto_scroll(20, 350),
            };

            if let Some(h) = hwnd {
                restore_window(h);
            }

            match result {
                Ok(img) => {
                    let path =
                        std::env::temp_dir().join(format!("yimage-capture-{}.png", unix_millis()));
                    if let Err(e) = crate::io::save::save_image(&img, &path) {
                        let _ = tx.send(BgMsg::Error(format!("save capture: {e:#}")));
                        return;
                    }
                    let _ = tx.send(BgMsg::ImageLoaded {
                        path,
                        image: img,
                        new_tab: true,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::Error(format!("capture: {e:#}")));
                }
            }
        });
    }

    /// Capture a fullscreen screenshot in the background and, when it
    /// completes, post a `CaptureScreenshot` message so the UI thread can
    /// open the region-crop overlay.
    ///
    /// Minimizes yImage first so the screenshot shows the actual desktop.
    #[cfg(all(windows, feature = "capture"))]
    fn spawn_capture_screenshot_for_crop(&mut self, mode: crate::capture::CaptureMode) {
        let tx = self.tx.clone();
        let hwnd = self_hwnd();
        std::thread::spawn(move || {
            if let Some(h) = hwnd {
                minimize_window(h);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }

            let result = crate::capture::capture_primary_screen();

            if let Some(h) = hwnd {
                restore_window(h);
            }

            match result {
                Ok(img) => {
                    let _ = tx.send(BgMsg::CaptureScreenshot { image: img, mode });
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::Error(format!("capture: {e:#}")));
                }
            }
        });
    }

    #[cfg(all(windows, feature = "capture"))]
    pub fn apply_hotkeys(&mut self) {
        if let Some(reg) = self.hotkeys.as_mut() {
            reg.apply(&self.settings.hotkeys);
        }
    }

    fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BgMsg::ImageLoaded {
                    path,
                    image,
                    new_tab,
                } => {
                    if path.as_os_str().is_empty() {
                        // AI operation result — update in-place with undo support.
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            tab.doc.replace(image);
                            tab.texture_dirty = true;
                        }
                        self.dialog.obj_mask = None;
                        self.dialog.obj_mask_tex = None;
                    } else if new_tab || self.tabs.is_empty() {
                        let doc = Document::from_rgba(image, Some(path.clone()));
                        self.add_tab(doc);
                        self.status = self
                            .i18n
                            .t("status-loaded", &[("path", path.display().to_string())]);
                    } else {
                        // Replace current tab (folder navigation).
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            tab.doc =
                                Document::from_rgba(image, Some(path.clone()));
                            tab.texture_dirty = true;
                            tab.viewer.reset_view = true;
                        }
                        self.dialog.obj_mask = None;
                        self.dialog.obj_mask_tex = None;
                        self.status = self
                            .i18n
                            .t("status-loaded", &[("path", path.display().to_string())]);
                    }
                    if !path.as_os_str().is_empty() {
                        self.push_recent(&path);
                    }
                    self.progress = None;
                    self.apply_pending_action();
                    ctx.request_repaint();
                }
                BgMsg::ImageSaved(p) => {
                    self.status = self
                        .i18n
                        .t("status-saved", &[("path", p.display().to_string())]);
                }
                BgMsg::Progress(label, v) => {
                    self.progress = Some((label, v));
                    ctx.request_repaint();
                }
                BgMsg::Error(e) => {
                    self.status = format!("error: {e}");
                    self.progress = None;
                }
                BgMsg::Info(i) => {
                    self.status = i;
                }
                #[cfg(all(windows, feature = "capture"))]
                BgMsg::CaptureScreenshot { image, mode } => {
                    let mut state =
                        crate::ui::capture_overlay::RegionCropState::new(image, mode);
                    if matches!(mode, crate::capture::CaptureMode::FixedRegion { .. }) {
                        state.preset = self.dialog.fixed_region;
                    }
                    self.dialog.region_crop = Some(state);
                    ctx.request_repaint();
                }
            }
        }
    }

    #[cfg(all(windows, feature = "capture"))]
    fn poll_hotkeys(&mut self) {
        let Some(reg) = self.hotkeys.as_ref() else {
            return;
        };
        for action in reg.poll() {
            let mode = action.to_mode();
            self.trigger_capture(mode);
        }
    }
}

impl eframe::App for YImageApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.thumbs_visible = self.thumbs.visible;
        eframe::set_value(storage, "settings", &self.settings);
    }

    fn ui(&mut self, ui_root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui_root.ctx().clone();
        self.poll_messages(&ctx);
        #[cfg(all(windows, feature = "capture"))]
        self.poll_hotkeys();

        let dropped_path: Option<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()));
        if let Some(path) = dropped_path {
            let _ = self
                .tx
                .send(BgMsg::Info(format!("opening {}", path.display())));
            self.open_path(&path, true);
        }

        // Declaration order matters for egui panel layout. New minimal stack:
        // 1. Tab bar        — hamburger menu + document tabs.
        // 2. Ribbon toolbar — tool buttons grouped into named sections.
        // 3. Context toolbar — appears under the ribbon when a tool is active.
        // 4. Status bar      — full-width interactive strip at the bottom.
        // 5. Thumbnails      — optional bottom filmstrip (hidden by default).
        // 6. CentralPanel    — viewer fills whatever remains.
        ui::unified_header::show_tab_bar(&ctx, self);
        ui::unified_header::show_ribbon(&ctx, self);
        ui::context_toolbar::show(&ctx, self);
        ui::statusbar::show(&ctx, self);
        ui::thumbnails::show(&ctx, self);
        ui::viewer::show(&ctx, self);
        ui::dialogs::show(&ctx, self);

        // Capture overlays (countdown banner + region-crop selector)
        // render last so they land on top of every other panel.
        #[cfg(all(windows, feature = "capture"))]
        ui::capture_overlay::show(&ctx, self);

        // GIF builder — real OS-level window via show_viewport_immediate.
        // Must be called after all main-window panels are set up.
        ui::gif_timeline::show_viewport(&ctx, self);
    }
}

fn is_supported_image(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tif" | "tiff" | "avif")
    )
}

#[cfg(all(windows, feature = "capture"))]
pub(crate) fn unix_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Find the yImage window handle by enumerating windows owned by our process.
/// Returns `None` if no matching window is found (e.g. on non-Windows).
#[cfg(all(windows, feature = "capture"))]
fn self_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == HWND::default() {
        None
    } else {
        Some(hwnd)
    }
}

#[cfg(all(windows, feature = "capture"))]
fn minimize_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
}

#[cfg(all(windows, feature = "capture"))]
fn restore_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        // Explicitly re-activate yImage so the region-crop overlay is
        // actually visible — otherwise SW_RESTORE may leave us behind the
        // window that gained focus while we were minimized, and the overlay
        // would render to a hidden window.
        let _ = SetForegroundWindow(hwnd);
    }
}
