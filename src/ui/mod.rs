pub mod dialogs;
pub mod sidebar;
pub mod statusbar;
pub mod toolbar;
pub mod viewer;

/// Install CJK-capable fonts so Korean and Japanese labels render correctly.
/// We bundle a Noto Sans CJK subset in `assets/fonts/` and fall through to the
/// egui defaults if it's missing (so `cargo check` on a clean tree works).
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let cjk_path = crate::assets_dir().join("fonts").join("NotoSansCJK.otf");
    if cjk_path.exists() {
        if let Ok(bytes) = std::fs::read(&cjk_path) {
            fonts.font_data.insert(
                "noto_cjk".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            // Prepend so CJK glyphs take priority for both proportional and
            // monospace families.
            if let Some(list) = fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
            {
                list.insert(0, "noto_cjk".to_owned());
            }
            if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                list.push("noto_cjk".to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}
