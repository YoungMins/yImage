// Tab bar showing open documents. Each tab displays the filename
// (or "Untitled" for unnamed captures), a dirty indicator, and a
// close button. Clicking a tab switches to it.

use crate::app::YImageApp;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    if app.tabs.is_empty() {
        return;
    }

    // Apple-style tab strip: pill-shaped tabs, subtle hover fill, accent
    // underline on the active tab. The bar sits flush with the toolbar so
    // the whole header reads as a single piece of chrome.
    egui::TopBottomPanel::top("tabbar")
        .exact_height(36.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_centered(|ui| {
                ui.add_space(6.0);
                ui.spacing_mut().item_spacing.x = 4.0;

                let mut switch_to: Option<usize> = None;
                let mut close_idx: Option<usize> = None;

                for (i, tab) in app.tabs.iter().enumerate() {
                    let is_active = i == app.active_tab;
                    let title = tab.title();
                    let dirty_marker = if tab.doc.dirty { "\u{25CF} " } else { "" };

                    let fill = if is_active {
                        ui.visuals().widgets.hovered.bg_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let frame = egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(12, 4))
                        .corner_radius(egui::CornerRadius::same(8))
                        .fill(fill);

                    let r = frame
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let text_color = if is_active {
                                    ui.visuals().strong_text_color()
                                } else {
                                    ui.visuals().weak_text_color()
                                };
                                ui.label(
                                    egui::RichText::new(format!("{dirty_marker}{title}"))
                                        .size(12.5)
                                        .color(text_color),
                                );
                                // Tiny round close button, only emphasised on hover.
                                let close_resp = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("\u{00D7}").size(12.0).color(
                                            ui.visuals().text_color().linear_multiply(0.55),
                                        ),
                                    )
                                    .frame(false)
                                    .min_size(egui::Vec2::new(14.0, 14.0)),
                                );
                                if close_resp.clicked() {
                                    close_idx = Some(i);
                                }
                            });
                        })
                        .response;

                    // Accent underline for the active tab — macOS-style
                    // indicator rather than a heavy border around the pill.
                    if is_active {
                        let y = r.rect.max.y - 1.0;
                        ui.painter().line_segment(
                            [
                                egui::pos2(r.rect.min.x + 10.0, y),
                                egui::pos2(r.rect.max.x - 10.0, y),
                            ],
                            egui::Stroke::new(2.0, super::theme::ACCENT),
                        );
                    }

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
