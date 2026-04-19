#[cfg(all(windows, feature = "capture"))]
pub mod capture_overlay;
pub mod context_toolbar;
pub mod dialogs;
pub mod gif_timeline;
pub mod statusbar;
pub mod theme;
pub mod thumbnails;
pub mod unified_header;
pub mod viewer;

// Noto Sans CJK JP (Regular) is embedded directly into the binary so the
// installed app always has a CJK-capable glyph source — no runtime file
// dependency, no broken glyphs if the installer skips the fonts folder.
// The JP region font also covers Hangul and CJK Unified so the same file
// renders Korean, Japanese, and Chinese correctly. It is also reused by the
// text-stamping tool (see tools::draw::TextState) so the raster text path
// can draw CJK out of the box.
pub(crate) const CJK_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/NotoSansCJK.otf");

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

    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        list.insert(0, "noto_cjk".to_owned());
    }
    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        list.push("noto_cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// Build an `egui::ColorImage` from an `image::RgbaImage` using the fastest
/// available path:
///
/// - **Opaque fast path**: when every alpha byte is 255, premultiplied and
///   unmultiplied RGBA are bit-identical, so we reinterpret the raw byte
///   buffer as `&[Color32]` and bulk-copy into the pixel vec. A 24 MP JPEG
///   goes from ~24 M `from_rgba_unmultiplied` calls to a single `memcpy`.
/// - **Alpha fallback**: if any pixel has partial alpha we premultiply pixel
///   by pixel, same as before.
pub(crate) fn rgba_to_color_image(img: &image::RgbaImage) -> egui::ColorImage {
    let size = [img.width() as usize, img.height() as usize];
    let raw: &[u8] = img.as_raw();
    let opaque = !raw.chunks_exact(4).any(|c| c[3] != 255);
    let pixels: Vec<egui::Color32> = if opaque {
        bytemuck::cast_slice::<u8, egui::Color32>(raw).to_vec()
    } else {
        raw.chunks_exact(4)
            .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .collect()
    };
    egui::ColorImage {
        size,
        source_size: egui::vec2(size[0] as f32, size[1] as f32),
        pixels,
    }
}
