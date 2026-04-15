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

/// Messages background workers can post to the UI thread.
#[derive(Debug)]
pub enum BgMsg {
    ImageLoaded {
        path: PathBuf,
        image: image::RgbaImage,
    },
    ImageSaved(PathBuf),
    Progress(String, f32),
    Error(String),
    Info(String),
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
            thumbs_visible: true,
            #[cfg(all(windows, feature = "capture"))]
            hotkeys: crate::hotkeys::defaults(),
            #[cfg(not(all(windows, feature = "capture")))]
            hotkeys: HashMap::new(),
        }
    }
}

pub struct YImageApp {
    pub doc: Option<Document>,
    pub tool: ToolKind,
    pub i18n: I18n,
    pub settings: Settings,
    pub status: String,
    pub progress: Option<(String, f32)>,
    pub tx: Sender<BgMsg>,
    pub rx: Receiver<BgMsg>,
    pub viewer: ui::viewer::ViewerState,
    pub dialog: ui::dialogs::DialogState,
    pub texture: Option<egui::TextureHandle>,
    pub texture_dirty: bool,
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
        if settings.theme_dark {
            ui::theme::apply_dark(&cc.egui_ctx);
        } else {
            ui::theme::apply_light(&cc.egui_ctx);
        }

        let (tx, rx) = crossbeam_channel::unbounded();

        let mut thumbs = ui::thumbnails::Thumbnails::new();
        thumbs.visible = settings.thumbs_visible;

        let mut app = Self {
            doc: None,
            tool: ToolKind::None,
            i18n,
            settings,
            status: String::new(),
            progress: None,
            tx,
            rx,
            viewer: ui::viewer::ViewerState::default(),
            dialog: ui::dialogs::DialogState::default(),
            texture: None,
            texture_dirty: false,
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
            app.open_path(&path);
        }

        app
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
                if let Some(doc) = &self.doc {
                    self.dialog.resize_w = doc.width();
                    self.dialog.resize_h = doc.height();
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
        let Some(doc) = self.doc.as_ref() else { return };
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
                    });
                    let _ = tx.send(BgMsg::Progress(label, 1.0));
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
        let Some(doc) = self.doc.as_ref() else { return };
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
                    });
                    let _ = tx.send(BgMsg::Progress(label, 1.0));
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

    pub fn open_path(&mut self, path: &Path) {
        let tx = self.tx.clone();
        let path = path.to_path_buf();
        self.settings.last_folder = path.parent().map(Path::to_path_buf);
        self.scan_folder(&path);
        rayon::spawn(move || match load_image(&path) {
            Ok(img) => {
                let _ = tx.send(BgMsg::ImageLoaded { path, image: img });
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
            .doc
            .as_ref()
            .and_then(|d| d.path.as_ref())
            .and_then(|p| entries.iter().position(|e| e == p))
            .unwrap_or(self.folder_index);
        let len = entries.len() as isize;
        let next = ((current as isize + delta).rem_euclid(len)) as usize;
        self.folder_index = next;
        let path = entries[next].clone();
        self.open_path(&path);
    }

    #[cfg(all(windows, feature = "capture"))]
    pub fn trigger_capture(&mut self, mode: crate::capture::CaptureMode) {
        use crate::capture::CaptureMode;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = match mode {
                CaptureMode::Fullscreen => crate::capture::capture_primary_screen(),
                CaptureMode::ActiveWindow => crate::capture::capture_active_window(),
                CaptureMode::Region => crate::capture::capture_primary_screen(),
                CaptureMode::FixedRegion { x, y, w, h } => {
                    crate::capture::capture_fixed_region(x, y, w, h)
                }
                CaptureMode::AutoScroll => crate::capture::capture_auto_scroll(20, 350),
            };
            match result {
                Ok(img) => {
                    let path =
                        std::env::temp_dir().join(format!("yimage-capture-{}.png", unix_millis()));
                    if let Err(e) = crate::io::save::save_image(&img, &path) {
                        let _ = tx.send(BgMsg::Error(format!("save capture: {e:#}")));
                        return;
                    }
                    let _ = tx.send(BgMsg::ImageLoaded { path, image: img });
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
                BgMsg::ImageLoaded { path, image } => {
                    let keep_path = if path.as_os_str().is_empty() {
                        self.doc.as_ref().and_then(|d| d.path.clone())
                    } else {
                        Some(path.clone())
                    };
                    let doc = Document::from_rgba(image, keep_path.clone());
                    self.doc = Some(doc);
                    self.texture_dirty = true;
                    if let Some(p) = keep_path.as_ref() {
                        self.status = self
                            .i18n
                            .t("status-loaded", &[("path", p.display().to_string())]);
                    }
                    self.viewer.reset_view = true;
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
            self.open_path(&path);
        }

        // Declaration order matters for egui panel layout:
        // 1. Top panel  → occupies full width at top.
        // 2. Bottom panel → occupies full width at bottom (must come before side panels).
        // 3. Left side panels (outermost → innermost, i.e. tool rail first, then thumbnails).
        // 4. Right side panel.
        // 5. CentralPanel → fills whatever remains.
        ui::toolbar::show(&ctx, self);
        ui::statusbar::show(&ctx, self);
        ui::toolpanel::show(&ctx, self);
        ui::thumbnails::show(&ctx, self);
        ui::sidebar::show(&ctx, self);
        ui::viewer::show(&ctx, self);
        ui::dialogs::show(&ctx, self);
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
fn unix_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
