// Top-level application state. Holds the current document, UI state, tool
// selection, i18n bundle, settings, and the channel used by background workers
// to push finished work back to the UI thread.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use parking_lot::Mutex;

use crate::document::Document;
use crate::i18n::I18n;
use crate::io::load::load_image;
use crate::tools::ToolKind;
use crate::ui;

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

/// Optional startup action derived from CLI flags. Lets Windows Explorer's
/// right-click verbs jump directly into a specific dialog after opening the
/// selected file.
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
    /// Action to auto-run as soon as the startup image finishes loading.
    /// Cleared once applied so subsequent user-initiated opens don't re-trigger.
    pub pending_action: StartupAction,
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
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
        }

        let (tx, rx) = crossbeam_channel::unbounded();

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
        };

        if let Some(path) = startup_file {
            app.open_path(&path);
        }

        app
    }

    /// Apply a pending Explorer-shell action once the image is actually
    /// loaded into the document. Clears `pending_action` so it only fires
    /// once per startup.
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

    /// Run U²-Net on the current document off the UI thread. Wired from both
    /// the sidebar "Run" button and the `--bg-remove` startup action.
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
                let _ = tx.send(BgMsg::Error(
                    "built without the `ai` feature".to_string(),
                ));
            }
        });
    }

    /// Load an image from disk asynchronously on a rayon worker.
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

    /// Populate `folder_entries` with neighbouring images so next/prev works.
    fn scan_folder(&self, current: &Path) {
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

    /// Drain background messages into UI-visible state.
    fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BgMsg::ImageLoaded { path, image } => {
                    // An empty path means "in-memory result" (e.g. bg-remove
                    // output); preserve the existing document path in that
                    // case so saves still go to the original file.
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
}

impl eframe::App for YImageApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }

    fn ui(&mut self, ui_root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui_root.ctx().clone();
        self.poll_messages(&ctx);

        // Accept drag-and-drop files.
        let dropped_path: Option<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .first()
                .and_then(|f| f.path.clone())
        });
        if let Some(path) = dropped_path {
            let _ = self
                .tx
                .send(BgMsg::Info(format!("opening {}", path.display())));
            self.open_path(&path);
        }

        ui::toolbar::show(&ctx, self);
        ui::sidebar::show(&ctx, self);
        ui::statusbar::show(&ctx, self);
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
        Some(
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tif" | "tiff" | "avif"
        )
    )
}
