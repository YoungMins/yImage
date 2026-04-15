pub mod dialogs;
pub mod sidebar;
pub mod statusbar;
pub mod toolbar;
pub mod viewer;

// Noto Sans CJK JP (Regular) is embedded directly into the binary so the
// installed app always has a CJK-capable glyph source — no runtime file
// dependency, no broken glyphs if the installer skips the fonts folder.
// The JP region font also covers Hangul and CJK Unified so the same file
// renders Korean, Japanese, and Chinese correctly.
const CJK_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/NotoSansCJK.otf");

/// Install CJK-capable fonts so Korean and Japanese labels render correctly.
/// The Noto Sans CJK font is embedded at compile time and inserted at the
/// top of both the Proportional and Monospace font fallback chains so any
/// character that egui's built-in Latin font can't draw falls through to it.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "noto_cjk".to_owned(),
        egui::FontData::from_static(CJK_FONT_BYTES).into(),
    );

    // Prepend on Proportional so CJK takes priority for UI labels, and
    // append on Monospace so code/path text still prefers the monospace
    // default but can fall through to CJK when needed.
    if let Some(list) = fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
    {
        list.insert(0, "noto_cjk".to_owned());
    }
    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        list.push("noto_cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}
