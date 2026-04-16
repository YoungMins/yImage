// Left-side vertical tool palette.
//
// A narrow, non-resizable rail that shows one icon button per editing tool.
// The active tool is highlighted with the accent colour. Hovering any button
// shows a tooltip with the full tool name so users can learn the icons without
// wasting horizontal space on labels.

use egui::{Color32, RichText, Vec2};

use crate::app::YImageApp;
use crate::tools::ToolKind;

/// Width of the tool-palette rail in logical pixels.
pub const PANEL_WIDTH: f32 = 56.0;

/// Icon button size inside the rail.
const BTN: f32 = 40.0;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::SidePanel::left("toolpanel")
        .resizable(false)
        .default_width(PANEL_WIDTH)
        .min_width(PANEL_WIDTH)
        .max_width(PANEL_WIDTH)
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                tool_btn(ui, app, ToolKind::None, "\u{2196}", "tool-none");
                group_divider(ui);
                tool_btn(ui, app, ToolKind::Draw, "\u{270F}", "tool-draw");
                tool_btn(ui, app, ToolKind::Mosaic, "\u{25A3}", "tool-mosaic");
                tool_btn(ui, app, ToolKind::Text, "A", "tool-text");
                tool_btn(ui, app, ToolKind::Shape, "\u{25FB}", "tool-shape");
                group_divider(ui);
                tool_btn(ui, app, ToolKind::BackgroundRemove, "\u{2702}", "tool-bg-remove");
                tool_btn(ui, app, ToolKind::ObjectRemove, "\u{2296}", "tool-obj-remove");
                group_divider(ui);
                tool_btn(ui, app, ToolKind::Gif, "\u{25B6}", "tool-gif");
            });
        });
}

/// Draw a thin centred hairline as a group divider. Apple uses short, offset
/// hairlines between icon groups rather than full-width separators.
fn group_divider(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(BTN - 8.0, 1.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    ui.add_space(6.0);
}

fn tool_btn(ui: &mut egui::Ui, app: &mut YImageApp, kind: ToolKind, icon: &str, label_key: &str) {
    let is_active = app.tool == kind;
    let label = app.i18n.t(label_key, &[]);

    let fill = if is_active {
        super::theme::ACCENT
    } else {
        Color32::TRANSPARENT
    };
    let text_color = if is_active {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };

    // Rounded-square icon tile, Apple-style. Accent fill for the active
    // tool gives an iOS-like "selected" state without any stroke.
    let btn = egui::Button::new(RichText::new(icon).size(18.0).color(text_color))
        .min_size(Vec2::new(BTN, BTN))
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(10));

    let r = ui.add(btn).on_hover_text(&label);
    if r.clicked() {
        app.tool = kind;
    }
    ui.add_space(4.0);
}
