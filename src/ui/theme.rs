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

/// Apply an Apple-inspired dark theme.
pub fn apply_dark(ctx: &egui::Context) {
    let mut style = Style::default();
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

    // Apple spacing: airy, consistent 10–12px gutters.
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.menu_margin = Margin::symmetric(8, 6);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.indent = 20.0;
    style.spacing.slider_width = 180.0;
    style.spacing.interact_size = Vec2::new(36.0, 30.0);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_spacing = 6.0;

    style.visuals = v;
    ctx.set_style(style);
}

/// Apply an Apple-inspired light theme (macOS Ventura / Sonoma feel).
pub fn apply_light(ctx: &egui::Context) {
    let mut style = Style::default();
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

    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.menu_margin = Margin::symmetric(8, 6);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.indent = 20.0;
    style.spacing.slider_width = 180.0;
    style.spacing.interact_size = Vec2::new(36.0, 30.0);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_spacing = 6.0;

    style.visuals = v;
    ctx.set_style(style);
}
