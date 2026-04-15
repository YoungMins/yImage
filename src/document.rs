// A Document owns the current image, its layer stack, and an undo history.
//
// The MVP is single-layer: the active image is the base; edits either apply
// destructively and push a snapshot onto the undo stack, or live on a temporary
// overlay buffer until committed. This keeps the memory model simple while
// still letting us expose Undo/Redo.

use std::path::PathBuf;

use image::RgbaImage;

pub const MAX_HISTORY: usize = 24;

pub struct Document {
    pub path: Option<PathBuf>,
    pub image: RgbaImage,
    /// Snapshots before each destructive edit. Newest at the end.
    undo_stack: Vec<RgbaImage>,
    /// Snapshots that were undone and can be redone.
    redo_stack: Vec<RgbaImage>,
    pub dirty: bool,
}

impl Document {
    pub fn from_rgba(image: RgbaImage, path: Option<PathBuf>) -> Self {
        Self {
            path,
            image,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    /// Snapshot the current image onto the undo stack before the caller
    /// mutates `self.image` in place.
    pub fn push_undo(&mut self) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.image.clone());
        self.dirty = true;
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack
                .push(std::mem::replace(&mut self.image, prev));
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack
                .push(std::mem::replace(&mut self.image, next));
            true
        } else {
            false
        }
    }

    pub fn replace(&mut self, new_image: RgbaImage) {
        self.push_undo();
        self.image = new_image;
    }
}
