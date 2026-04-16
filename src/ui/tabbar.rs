// Tab bar showing open documents. Each tab displays the filename
// (or "Untitled" for unnamed captures), a dirty indicator, and a
// close button. Clicking a tab switches to it.

use crate::app::YImageApp;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if app.tabs.is_empty() {
        return;
    }

    egui::TopBottomPanel::top("tabbar")
        .exact_height(28.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let mut switch_to: Option<usize> = None;
                let mut close_idx: Option<usize> = None;

                for (i, tab) in app.tabs.iter().enumerate() {
                    let is_active = i == app.active_tab;
                    let title = tab.title();
                    let dirty_marker = if tab.doc.dirty { "\u{25CF} " } else { "" };

                    let accent = super::theme::ACCENT;
                    let fill = if is_active {
                        accent.linear_multiply(0.15)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let stroke = if is_active {
                        egui::Stroke::new(1.0, accent)
                    } else {
                        egui::Stroke::NONE
                    };

                    let frame = egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .corner_radius(egui::CornerRadius::same(4))
                        .fill(fill)
                        .stroke(stroke);

                    let r = frame
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let text_color = if is_active {
                                    accent
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(
                                    egui::RichText::new(format!("{dirty_marker}{title}"))
                                        .small()
                                        .color(text_color),
                                );
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("\u{00D7}").small().color(
                                                ui.visuals().text_color().linear_multiply(0.6),
                                            ),
                                        )
                                        .frame(false)
                                        .min_size(egui::Vec2::new(16.0, 16.0)),
                                    )
                                    .clicked()
                                {
                                    close_idx = Some(i);
                                }
                            });
                        })
                        .response;

                    if r.interact(egui::Sense::click()).clicked() && close_idx.is_none() {
                        switch_to = Some(i);
                    }
                }

                if let Some(idx) = switch_to {
                    if idx != app.active_tab {
                        app.dialog.obj_mask = None;
                    }
                    app.active_tab = idx;
                }
                if let Some(idx) = close_idx {
                    app.close_tab(idx);
                }
            });
        });
}
