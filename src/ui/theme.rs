// WinUI 3 (Fluent Design) inspired egui theme.
//
// Reproduces the look & feel of Windows 11's Mica / Fluent surfaces using
// egui's `Visuals` API: neutral grey backgrounds, rounded corners, subtle
// stroke accents, and a single Microsoft-blue accent colour. We don't try
// to ape every Fluent effect (acrylic, reveal highlight) — just the parts
// egui can express cheaply so the app reads as "native modern Windows"
// rather than the default egui grey.

use egui::{
    epaint::Shadow, style::Selection, Color32, CornerRadius, Margin, Stroke, Style, Vec2, Visuals,
};

/// WinUI 3 accent blue (SystemAccentColor, dark shade 1).
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x78, 0xD4);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x1A, 0x88, 0xDD);
pub const ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x00, 0x5A, 0x9E);

/// Apply a Fluent-inspired dark theme to an egui context.
pub fn apply_dark(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut v = Visuals::dark();

    // Background layers: a near-black base with slightly lighter panels
    // to approximate WinUI's elevation hierarchy.
    v.panel_fill = Color32::from_rgb(0x1F, 0x1F, 0x1F);
    v.window_fill = Color32::from_rgb(0x2B, 0x2B, 0x2B);
    v.extreme_bg_color = Color32::from_rgb(0x17, 0x17, 0x17);
    v.faint_bg_color = Color32::from_rgb(0x26, 0x26, 0x26);
    v.code_bg_color = Color32::from_rgb(0x1A, 0x1A, 0x1A);

    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3A, 0x3A));
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };

    // Widget palette. WinUI uses a narrow stroke with rounded corners and
    // a hover tint; we translate that to egui's WidgetVisuals slots.
    let widgets = &mut v.widgets;

    widgets.noninteractive.bg_fill = v.panel_fill;
    widgets.noninteractive.weak_bg_fill = v.panel_fill;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x2E, 0x2E, 0x2E));
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xE6, 0xE6, 0xE6));
    widgets.noninteractive.corner_radius = CornerRadius::same(6);

    widgets.inactive.bg_fill = Color32::from_rgb(0x2D, 0x2D, 0x2D);
    widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2D, 0x2D, 0x2D);
    widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3A, 0x3A));
    widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xEA, 0xEA, 0xEA));
    widgets.inactive.corner_radius = CornerRadius::same(6);

    widgets.hovered.bg_fill = Color32::from_rgb(0x37, 0x37, 0x37);
    widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x33, 0x33);
    widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x52, 0x52, 0x52));
    widgets.hovered.fg_stroke = Stroke::new(1.2, Color32::from_rgb(0xFF, 0xFF, 0xFF));
    widgets.hovered.corner_radius = CornerRadius::same(6);

    widgets.active.bg_fill = Color32::from_rgb(0x45, 0x45, 0x45);
    widgets.active.weak_bg_fill = Color32::from_rgb(0x40, 0x40, 0x40);
    widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    widgets.active.fg_stroke = Stroke::new(1.2, Color32::WHITE);
    widgets.active.corner_radius = CornerRadius::same(6);

    widgets.open.bg_fill = Color32::from_rgb(0x33, 0x33, 0x33);
    widgets.open.weak_bg_fill = Color32::from_rgb(0x30, 0x30, 0x30);
    widgets.open.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x50, 0x50, 0x50));
    widgets.open.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xE6, 0xE6, 0xE6));
    widgets.open.corner_radius = CornerRadius::same(6);

    // Selection (highlighted text, selectable_value, progress bar fill).
    v.selection = Selection {
        bg_fill: ACCENT,
        stroke: Stroke::new(1.0, Color32::WHITE),
    };
    v.hyperlink_color = ACCENT_HOVER;

    // Spacing — WinUI uses slightly looser spacing than egui's default.
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.menu_margin = Margin::symmetric(6, 4);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.indent = 18.0;
    style.spacing.slider_width = 160.0;
    style.spacing.interact_size = Vec2::new(32.0, 28.0);

    style.visuals = v;
    ctx.set_style(style);
}

/// Apply a Fluent-inspired light theme.
pub fn apply_light(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut v = Visuals::light();

    v.panel_fill = Color32::from_rgb(0xF3, 0xF3, 0xF3);
    v.window_fill = Color32::from_rgb(0xFB, 0xFB, 0xFB);
    v.extreme_bg_color = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    v.faint_bg_color = Color32::from_rgb(0xEC, 0xEC, 0xEC);
    v.code_bg_color = Color32::from_rgb(0xF0, 0xF0, 0xF0);

    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0xD0, 0xD0, 0xD0));
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(40),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(30),
    };

    let widgets = &mut v.widgets;

    widgets.noninteractive.bg_fill = v.panel_fill;
    widgets.noninteractive.weak_bg_fill = v.panel_fill;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xDE, 0xDE, 0xDE));
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1F, 0x1F, 0x1F));
    widgets.noninteractive.corner_radius = CornerRadius::same(6);

    widgets.inactive.bg_fill = Color32::from_rgb(0xFB, 0xFB, 0xFB);
    widgets.inactive.weak_bg_fill = Color32::from_rgb(0xFB, 0xFB, 0xFB);
    widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xD4, 0xD4, 0xD4));
    widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1A, 0x1A, 0x1A));
    widgets.inactive.corner_radius = CornerRadius::same(6);

    widgets.hovered.bg_fill = Color32::from_rgb(0xF5, 0xF5, 0xF5);
    widgets.hovered.weak_bg_fill = Color32::from_rgb(0xEF, 0xEF, 0xEF);
    widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xC0, 0xC0, 0xC0));
    widgets.hovered.fg_stroke = Stroke::new(1.2, Color32::from_rgb(0x10, 0x10, 0x10));
    widgets.hovered.corner_radius = CornerRadius::same(6);

    widgets.active.bg_fill = Color32::from_rgb(0xE8, 0xE8, 0xE8);
    widgets.active.weak_bg_fill = Color32::from_rgb(0xE0, 0xE0, 0xE0);
    widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    widgets.active.fg_stroke = Stroke::new(1.2, Color32::BLACK);
    widgets.active.corner_radius = CornerRadius::same(6);

    widgets.open.bg_fill = Color32::from_rgb(0xF0, 0xF0, 0xF0);
    widgets.open.weak_bg_fill = Color32::from_rgb(0xEA, 0xEA, 0xEA);
    widgets.open.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xCC, 0xCC, 0xCC));
    widgets.open.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1A, 0x1A, 0x1A));
    widgets.open.corner_radius = CornerRadius::same(6);

    v.selection = Selection {
        bg_fill: ACCENT,
        stroke: Stroke::new(1.0, Color32::WHITE),
    };
    v.hyperlink_color = ACCENT;

    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.menu_margin = Margin::symmetric(6, 4);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.indent = 18.0;
    style.spacing.slider_width = 160.0;
    style.spacing.interact_size = Vec2::new(32.0, 28.0);

    style.visuals = v;
    ctx.set_style(style);
}
