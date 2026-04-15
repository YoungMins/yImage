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
pub const PANEL_WIDTH: f32 = 52.0;

/// Icon button size inside the rail.
const BTN: f32 = 36.0;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    egui::SidePanel::left("toolpanel")
        .resizable(false)
        .default_width(PANEL_WIDTH)
        .min_width(PANEL_WIDTH)
        .max_width(PANEL_WIDTH)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                tool_btn(ui, app, ToolKind::None, "↖", "tool-none");
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                tool_btn(ui, app, ToolKind::Draw, "✏", "tool-draw");
                tool_btn(ui, app, ToolKind::Mosaic, "▣", "tool-mosaic");
                tool_btn(ui, app, ToolKind::Text, "A", "tool-text");
                tool_btn(ui, app, ToolKind::Shape, "◻", "tool-shape");
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                tool_btn(ui, app, ToolKind::BackgroundRemove, "✂", "tool-bg-remove");
                tool_btn(ui, app, ToolKind::ObjectRemove, "⊖", "tool-obj-remove");
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                tool_btn(ui, app, ToolKind::Gif, "▶", "tool-gif");
            });
        });
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

    let btn = egui::Button::new(RichText::new(icon).size(17.0).color(text_color))
        .min_size(Vec2::new(BTN, BTN))
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(6));

    let r = ui.add(btn).on_hover_text(&label);
    if r.clicked() {
        app.tool = kind;
    }
    ui.add_space(2.0);
}
