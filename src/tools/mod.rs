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
    Text,
    Shape,
    BackgroundRemove,
    ObjectRemove,
    Gif,
}

impl ToolKind {
    /// True if the tool paints a cursor-preview circle over the viewer when
    /// the pointer hovers the image (brush, mosaic etc.).
    pub fn has_brush_preview(self) -> bool {
        matches!(
            self,
            ToolKind::Draw | ToolKind::Mosaic | ToolKind::ObjectRemove
        )
    }
}
