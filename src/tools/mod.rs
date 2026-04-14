// Editing tools. Each tool handles viewer input events and mutates the active
// Document. The current tool is stored on the App as a ToolKind discriminant;
// per-tool state lives on the App itself so nothing needs dynamic dispatch at
// the hot rendering path.

pub mod bg_remove;
pub mod draw;
pub mod mosaic;
pub mod obj_remove;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolKind {
    #[default]
    None,
    Draw,
    Mosaic,
    BackgroundRemove,
    ObjectRemove,
}
