// Apple-inspired (macOS / iOS style) egui theme.
//
// Reproduces the neutral, airy feel of macOS windows and iOS controls using
// egui's `Visuals` API: soft neutral greys, generous rounded corners, thin
// hairline strokes, subtle shadows, and Apple's system blue as the sole
// accent. The result is a flat, content-first look rather than the heavier
// Fluent-style chrome used previously.

use egui::{
    epaint::Shadow, style::Selection, Color32, CornerRadius, Margin, Stroke, Style, Vec2, Visuals,
};

/// Apple system blue. The light/dark shades match iOS/macOS `systemBlue`.
pub const ACCENT: Color32 = Color32::from_rgb(0x0A, 0x84, 0xFF); // dark-mode blue
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x26, 0x94, 0xFF);
pub const ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x00, 0x6F, 0xE0);

// Semantic colors — avoid scattered magic hex values across UI files.
pub const SURFACE_ELEVATED_DARK: Color32 = Color32::from_rgb(0x2C, 0x2C, 0x2E);
pub const SURFACE_ELEVATED_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const TEXT_SECONDARY_DARK: Color32 = Color32::from_rgb(0x8E, 0x8E, 0x93);
pub const TEXT_SECONDARY_LIGHT: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x73);
pub const DIVIDER_DARK: Color32 = Color32::from_rgb(0x38, 0x38, 0x3A);
pub const DIVIDER_LIGHT: Color32 = Color32::from_rgb(0xE5, 0xE5, 0xEA);
pub const SUCCESS: Color32 = Color32::from_rgb(0x30, 0xD1, 0x58);
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0x9F, 0x0A);

// Typography scale.
pub const FONT_DISPLAY: f32 = 28.0;
pub const FONT_TITLE: f32 = 16.0;
pub const FONT_BODY: f32 = 13.0;
pub const FONT_CAPTION: f32 = 11.5;
pub const FONT_TINY: f32 = 10.0;

// Spacing tokens.
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
pub const SPACE_XL: f32 = 24.0;

/// Pre-configured card frame for modal dialogs and floating panels.
pub fn card_frame(dark: bool) -> egui::Frame {
    let fill = if dark {
        SURFACE_ELEVATED_DARK
    } else {
        SURFACE_ELEVATED_LIGHT
    };
    let stroke_color = if dark { DIVIDER_DARK } else { DIVIDER_LIGHT };
    egui::Frame::none()
        .fill(fill)
        .inner_margin(Margin::same(20))
        .corner_radius(CornerRadius::same(14))
        .stroke(Stroke::new(1.0, stroke_color))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 140 } else { 40 }),
        })
}

/// Frame for the contextual toolbar that docks to the top of the canvas.
pub fn toolbar_frame(dark: bool) -> egui::Frame {
    let fill = if dark {
        Color32::from_rgb(0x24, 0x24, 0x26)
    } else {
        Color32::from_rgb(0xF8, 0xF8, 0xFA)
    };
    let stroke_color = if dark { DIVIDER_DARK } else { DIVIDER_LIGHT };
    egui::Frame::none()
        .fill(fill)
        .inner_margin(Margin::symmetric(14, 8))
        .corner_radius(CornerRadius {
            nw: 0,
            ne: 0,
            sw: 10,
            se: 10,
        })
        .stroke(Stroke::new(0.5, stroke_color))
        .shadow(Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 80 } else { 20 }),
        })
}

/// Render a small rounded pill badge (e.g. "PNG", "JPEG").
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let font = egui::FontId::proportional(FONT_TINY);
    let text_size = ui.painter().layout_no_wrap(text.to_string(), font.clone(), color).size();
    let desired = Vec2::new(text_size.x + 10.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let bg = color.linear_multiply(0.15);
    ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        color,
    );
}

/// Overlay our Apple-inspired spacing tokens onto an existing Style,
/// leaving every other field untouched. Used for both theme slots so
/// egui's own pre-configured fields (text styles, animation times, any
/// eframe integration defaults, etc.) stay intact — we only paint over
/// spacing and, separately, visuals.
fn apply_spacing(style: &mut Style) {
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.menu_margin = Margin::symmetric(8, 6);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.indent = 20.0;
    style.spacing.slider_width = 180.0;
    style.spacing.interact_size = Vec2::new(36.0, 30.0);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_spacing = 6.0;
}

/// Install the full theme. Paints our spacing + visuals onto BOTH of
/// egui's theme slots (`dark_style` / `light_style`) via in-place mutation
/// — not via full Style replacement — so any fields egui or eframe set up
/// for us (text styles, animation times, interaction radii, …) survive.
/// Then pins the active theme to the user's saved preference so the
/// OS-level "follow system" fallback can't silently override it on
/// restart.
pub fn install(ctx: &egui::Context, dark: bool) {
    ctx.all_styles_mut(apply_spacing);
    ctx.style_mut_of(egui::Theme::Dark, |s| s.visuals = build_dark_visuals());
    ctx.style_mut_of(egui::Theme::Light, |s| s.visuals = build_light_visuals());
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

fn build_dark_visuals() -> Visuals {
    let mut v = Visuals::dark();

    // macOS dark window palette. Content surfaces sit slightly above the
    // chrome so tabs/sidebars recede into the frame.
    v.panel_fill = Color32::from_rgb(0x1C, 0x1C, 0x1E); // systemGray6 dark
    v.window_fill = Color32::from_rgb(0x2C, 0x2C, 0x2E); // systemGray5 dark
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x14, 0x16);
    v.faint_bg_color = Color32::from_rgb(0x24, 0x24, 0x26);
    v.code_bg_color = Color32::from_rgb(0x18, 0x18, 0x1A);

    // Hairline strokes — Apple leans on 1px neutrals rather than colored
    // borders.
    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x38, 0x38, 0x3A));
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = Shadow {
        offset: [0, 14],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };

    let widgets = &mut v.widgets;

    widgets.noninteractive.bg_fill = v.panel_fill;
    widgets.noninteractive.weak_bg_fill = v.panel_fill;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x2E, 0x2E, 0x30));
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xEC, 0xEC, 0xEE));
    widgets.noninteractive.corner_radius = CornerRadius::same(8);

    // Apple buttons are almost-borderless pills that only show a fill on
    // hover/active. Keep the default fill subtle and lean on accent for
    // affordance.
    widgets.inactive.bg_fill = Color32::from_rgb(0x3A, 0x3A, 0x3C);
    widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2C, 0x2C, 0x2E);
    widgets.inactive.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xF2, 0xF2, 0xF7));
    widgets.inactive.corner_radius = CornerRadius::same(8);

    widgets.hovered.bg_fill = Color32::from_rgb(0x48, 0x48, 0x4A);
    widgets.hovered.weak_bg_fill = Color32::from_rgb(0x3A, 0x3A, 0x3C);
    widgets.hovered.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    widgets.hovered.corner_radius = CornerRadius::same(8);

    widgets.active.bg_fill = ACCENT;
    widgets.active.weak_bg_fill = Color32::from_rgb(0x3A, 0x3A, 0x3C);
    widgets.active.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    widgets.active.corner_radius = CornerRadius::same(8);

    widgets.open.bg_fill = Color32::from_rgb(0x3A, 0x3A, 0x3C);
    widgets.open.weak_bg_fill = Color32::from_rgb(0x2C, 0x2C, 0x2E);
    widgets.open.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.open.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xEC, 0xEC, 0xEE));
    widgets.open.corner_radius = CornerRadius::same(8);

    v.selection = Selection {
        bg_fill: Color32::from_rgba_unmultiplied(0x0A, 0x84, 0xFF, 150),
        stroke: Stroke::new(1.0, Color32::WHITE),
    };
    v.hyperlink_color = ACCENT_HOVER;

    v
}

/// Swap to the dark theme at runtime. Both dark_style and light_style were
/// pre-populated by `install`, so flipping the theme preference is all that
/// is needed — no Style mutation, no spacing shift, nothing but a visuals
/// switch egui already has primed.
pub fn apply_dark(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
}

fn build_light_visuals() -> Visuals {
    let mut v = Visuals::light();

    // macOS light palette — soft whites with a warm-grey undertone.
    v.panel_fill = Color32::from_rgb(0xF5, 0xF5, 0xF7); // systemGray6 light
    v.window_fill = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    v.extreme_bg_color = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    v.faint_bg_color = Color32::from_rgb(0xEC, 0xEC, 0xEE);
    v.code_bg_color = Color32::from_rgb(0xF2, 0xF2, 0xF7);

    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0xE5, 0xE5, 0xEA));
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = Shadow {
        offset: [0, 12],
        blur: 36,
        spread: 0,
        color: Color32::from_black_alpha(36),
    };
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(28),
    };

    let widgets = &mut v.widgets;

    widgets.noninteractive.bg_fill = v.panel_fill;
    widgets.noninteractive.weak_bg_fill = v.panel_fill;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xE5, 0xE5, 0xEA));
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1C, 0x1C, 0x1E));
    widgets.noninteractive.corner_radius = CornerRadius::same(8);

    widgets.inactive.bg_fill = Color32::from_rgb(0xEF, 0xEF, 0xF4);
    widgets.inactive.weak_bg_fill = Color32::from_rgb(0xF5, 0xF5, 0xF7);
    widgets.inactive.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1C, 0x1C, 0x1E));
    widgets.inactive.corner_radius = CornerRadius::same(8);

    widgets.hovered.bg_fill = Color32::from_rgb(0xE5, 0xE5, 0xEA);
    widgets.hovered.weak_bg_fill = Color32::from_rgb(0xEC, 0xEC, 0xEE);
    widgets.hovered.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x0F, 0x0F, 0x0F));
    widgets.hovered.corner_radius = CornerRadius::same(8);

    widgets.active.bg_fill = ACCENT;
    widgets.active.weak_bg_fill = Color32::from_rgb(0xE5, 0xE5, 0xEA);
    widgets.active.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    widgets.active.corner_radius = CornerRadius::same(8);

    widgets.open.bg_fill = Color32::from_rgb(0xEF, 0xEF, 0xF4);
    widgets.open.weak_bg_fill = Color32::from_rgb(0xE5, 0xE5, 0xEA);
    widgets.open.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
    widgets.open.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1C, 0x1C, 0x1E));
    widgets.open.corner_radius = CornerRadius::same(8);

    v.selection = Selection {
        bg_fill: Color32::from_rgba_unmultiplied(0x00, 0x7A, 0xFF, 120),
        stroke: Stroke::new(1.0, Color32::WHITE),
    };
    v.hyperlink_color = Color32::from_rgb(0x00, 0x7A, 0xFF);

    v
}

/// Swap to the light theme at runtime. Companion to `apply_dark`.
pub fn apply_light(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
}
