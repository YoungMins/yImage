// Global hotkey manager for screen capture.
//
// Wraps `global-hotkey` so the rest of the app can register/unregister
// hotkeys by name, look up which action a fired id belongs to, and detect
// conflicts up front before committing a new binding. Hotkeys are delivered
// on a background receiver which we drain once per frame from the UI loop.

#![cfg(all(windows, feature = "capture"))]

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{Context, Result};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};

use crate::capture::CaptureMode;

/// One of the capture actions the user can bind a hotkey to.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum HotkeyAction {
    CaptureFullscreen,
    CaptureActiveWindow,
    CaptureRegion,
    CaptureFixedRegion,
    CaptureAutoScroll,
}

impl HotkeyAction {
    pub fn as_key(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullscreen => "capture.fullscreen",
            HotkeyAction::CaptureActiveWindow => "capture.window",
            HotkeyAction::CaptureRegion => "capture.region",
            HotkeyAction::CaptureFixedRegion => "capture.fixed",
            HotkeyAction::CaptureAutoScroll => "capture.scroll",
        }
    }

    pub fn to_mode(self) -> CaptureMode {
        match self {
            HotkeyAction::CaptureFullscreen => CaptureMode::Fullscreen,
            HotkeyAction::CaptureActiveWindow => CaptureMode::ActiveWindow,
            HotkeyAction::CaptureRegion => CaptureMode::Region,
            HotkeyAction::CaptureFixedRegion => CaptureMode::FixedRegion {
                x: 0,
                y: 0,
                w: 800,
                h: 600,
            },
            HotkeyAction::CaptureAutoScroll => CaptureMode::AutoScroll,
        }
    }

    pub fn all() -> [HotkeyAction; 5] {
        [
            HotkeyAction::CaptureFullscreen,
            HotkeyAction::CaptureActiveWindow,
            HotkeyAction::CaptureRegion,
            HotkeyAction::CaptureFixedRegion,
            HotkeyAction::CaptureAutoScroll,
        ]
    }
}

/// Default keybindings the first time yImage runs. The user can reassign
/// them via the Inspector or Settings dialog; changes are persisted to the
/// application settings blob.
pub fn defaults() -> HashMap<HotkeyAction, String> {
    let mut m = HashMap::new();
    m.insert(HotkeyAction::CaptureFullscreen, "PrintScreen".to_string());
    m.insert(
        HotkeyAction::CaptureActiveWindow,
        "Alt+PrintScreen".to_string(),
    );
    m.insert(HotkeyAction::CaptureRegion, "Ctrl+Shift+KeyA".to_string());
    m.insert(
        HotkeyAction::CaptureFixedRegion,
        "Ctrl+Shift+KeyF".to_string(),
    );
    m.insert(
        HotkeyAction::CaptureAutoScroll,
        "Ctrl+Shift+KeyS".to_string(),
    );
    m
}

/// Runtime hotkey state: a `GlobalHotKeyManager` + a reverse index so we can
/// map a fired `HotKey::id()` back to the action it corresponds to.
pub struct HotkeyRegistry {
    pub manager: GlobalHotKeyManager,
    pub bindings: HashMap<HotkeyAction, (HotKey, String)>,
    pub id_to_action: HashMap<u32, HotkeyAction>,
    /// Errors per action from the last apply() call. `None` means "registered
    /// successfully"; `Some(msg)` describes why it failed (conflict, parse).
    pub errors: HashMap<HotkeyAction, String>,
}

impl HotkeyRegistry {
    pub fn new() -> Result<Self> {
        Ok(Self {
            manager: GlobalHotKeyManager::new().context("create global hotkey manager")?,
            bindings: HashMap::new(),
            id_to_action: HashMap::new(),
            errors: HashMap::new(),
        })
    }

    /// Clear and re-register every binding from `config`. Failures are
    /// recorded per action in `self.errors` but don't abort the others.
    pub fn apply(&mut self, config: &HashMap<HotkeyAction, String>) {
        // Unregister everything currently bound.
        for (_, (hk, _)) in self.bindings.drain() {
            let _ = self.manager.unregister(hk);
        }
        self.id_to_action.clear();
        self.errors.clear();

        for action in HotkeyAction::all() {
            let Some(spec) = config.get(&action) else {
                continue;
            };
            if spec.is_empty() {
                continue;
            }
            match HotKey::from_str(spec) {
                Ok(hk) => {
                    let id = hk.id();
                    match self.manager.register(hk) {
                        Ok(()) => {
                            self.id_to_action.insert(id, action);
                            self.bindings.insert(action, (hk, spec.clone()));
                        }
                        Err(e) => {
                            self.errors.insert(action, format!("{e}"));
                        }
                    }
                }
                Err(e) => {
                    self.errors.insert(action, format!("parse: {e}"));
                }
            }
        }
    }

    /// Detect conflicts within `config` itself (two different actions bound
    /// to the same string). Returns a map action → other-action-it-conflicts-with
    /// so the UI can warn the user before committing.
    pub fn detect_conflicts(
        config: &HashMap<HotkeyAction, String>,
    ) -> HashMap<HotkeyAction, HotkeyAction> {
        let mut by_spec: HashMap<String, HotkeyAction> = HashMap::new();
        let mut conflicts = HashMap::new();
        for (action, spec) in config.iter() {
            let s = spec.trim();
            if s.is_empty() {
                continue;
            }
            if let Some(existing) = by_spec.get(s).copied() {
                conflicts.insert(*action, existing);
                conflicts.insert(existing, *action);
            } else {
                by_spec.insert(s.to_string(), *action);
            }
        }
        conflicts
    }

    pub fn poll(&self) -> Vec<HotkeyAction> {
        let rx = GlobalHotKeyEvent::receiver();
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if ev.state() == global_hotkey::HotKeyState::Pressed {
                if let Some(a) = self.id_to_action.get(&ev.id()).copied() {
                    out.push(a);
                }
            }
        }
        out
    }
}
