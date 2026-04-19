// Bottom status bar: interactive zoom + image metadata + status.
//
// Three zones laid out left-to-right across a single slim strip:
//   · Left   — dimensions, format badge, file size.
//   · Center — zoom: [-] [slider] [+] and a clickable percentage that cycles
//              through common presets (25/50/100/200/Fit).
//   · Right  — status message followed by an info `(i)` button that opens a
//              popover with full image properties (replaces the old Inspector
//              side panel).

use egui::{Color32, Sense, Stroke, Vec2};

use crate::app::YImageApp;
use crate::ui::theme;

pub fn show(ctx: &egui::Context, app: &mut YImageApp) {
    let dark = ctx.style().visuals.dark_mode;
    let bar_frame = egui::Frame::none()
        .fill(if dark { theme::GRADIENT_BOT_DARK } else { theme::GRADIENT_BOT_LIGHT })
        .stroke(egui::Stroke::NONE);

    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(30.0)
        .frame(bar_frame)
        .show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(10.0);
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // ── Left zone: dimensions + format badge + size ────
                    left_zone(ui, app);

                    // ── Right zone (laid out from the right edge) ──────
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add_space(10.0);
                            right_zone(ui, app);

                            // ── Center zone fills the remaining space ──
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    center_zone(ui, app);
                                },
                            );
                        },
                    );
                },
            );
        });
}

fn left_zone(ui: &mut egui::Ui, app: &YImageApp) {
    let Some(doc) = app.active_doc() else {
        ui.weak(egui::RichText::new("—").size(theme::FONT_CAPTION));
        return;
    };
    let w = doc.width();
    let h = doc.height();
    ui.label(
        egui::RichText::new(format!("{w} × {h}"))
            .size(theme::FONT_CAPTION)
            .color(text_secondary(app)),
    );

    // Format badge (e.g. "PNG"), if we know the file extension.
    if let Some(path) = &doc.path {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let up = ext.to_ascii_uppercase();
            if !up.is_empty() {
                theme::badge(ui, &up, theme::ACCENT);
            }
        }
        if let Ok(meta) = std::fs::metadata(path) {
            ui.label(
                egui::RichText::new(format_bytes(meta.len()))
                    .size(theme::FONT_CAPTION)
                    .color(text_secondary(app)),
            );
        }
    }
}

fn center_zone(ui: &mut egui::Ui, app: &mut YImageApp) {
    let idx = app.active_tab;
    if app.tabs.is_empty() || idx >= app.tabs.len() {
        return;
    }

    // A compact cluster: [-] slider [+] 100%.
    let mut zoom = app.tabs[idx].viewer.zoom.max(0.02);
    let mut changed = false;

    if ui
        .add(egui::Button::new(egui::RichText::new("−").size(14.0)).min_size(Vec2::new(22.0, 22.0)))
        .on_hover_text("Zoom out")
        .clicked()
    {
        zoom = (zoom * 0.8).clamp(0.02, 32.0);
        changed = true;
    }

    ui.spacing_mut().slider_width = 140.0;
    let slider_resp = ui.add(
        egui::Slider::new(&mut zoom, 0.02..=32.0)
            .logarithmic(true)
            .show_value(false),
    );
    if slider_resp.changed() {
        changed = true;
    }

    if ui
        .add(egui::Button::new(egui::RichText::new("+").size(14.0)).min_size(Vec2::new(22.0, 22.0)))
        .on_hover_text("Zoom in")
        .clicked()
    {
        zoom = (zoom * 1.25).clamp(0.02, 32.0);
        changed = true;
    }

    // Clickable percentage: cycles through Fit → 25 → 50 → 100 → 200.
    let pct_text = format!("{:>4.0}%", zoom * 100.0);
    let pct_resp = ui.add(
        egui::Label::new(
            egui::RichText::new(pct_text)
                .size(theme::FONT_CAPTION)
                .monospace()
                .color(ui.visuals().text_color()),
        )
        .sense(Sense::click()),
    );
    if pct_resp.hovered() {
        ui.painter().rect_filled(
            pct_resp.rect.expand(2.0),
            egui::CornerRadius::same(4),
            theme::ACCENT.linear_multiply(0.08),
        );
    }
    if pct_resp.clicked() {
        // Cycle through sensible preset zooms.
        let presets = [0.25_f32, 0.5, 1.0, 2.0];
        let next = presets
            .iter()
            .copied()
            .find(|&p| p > zoom + 0.001)
            .unwrap_or(presets[0]);
        zoom = next;
        changed = true;
    }
    if pct_resp.secondary_clicked() || pct_resp.middle_clicked() {
        app.tabs[idx].viewer.reset_view = true;
    }

    if changed {
        app.tabs[idx].viewer.zoom = zoom;
    }
}

fn right_zone(ui: &mut egui::Ui, app: &mut YImageApp) {
    // Info popover (opens a panel with full image properties).
    let info_btn = ui.add(
        egui::Button::new(egui::RichText::new("ⓘ").size(14.0))
            .min_size(Vec2::new(22.0, 22.0))
            .frame(false),
    );
    egui::Popup::from_toggle_button_response(&info_btn)
        .align(egui::RectAlign::TOP)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui: &mut egui::Ui| {
            ui.set_min_width(260.0);
            image_properties_popover(ui, app);
        });

    // Status message (left of the info button, since we're in a R→L layout).
    let status = if app.status.is_empty() {
        app.i18n.t("welcome-open", &[])
    } else {
        app.status.clone()
    };
    ui.add(
        egui::Label::new(
            egui::RichText::new(status)
                .size(theme::FONT_CAPTION)
                .color(text_secondary(app)),
        )
        .truncate(),
    );
}

fn image_properties_popover(ui: &mut egui::Ui, app: &YImageApp) {
    ui.strong(
        egui::RichText::new(app.i18n.t("inspector-properties", &[])).size(theme::FONT_TITLE),
    );
    ui.add_space(theme::SPACE_XS);
    let divider = if app.settings.theme_dark {
        theme::DIVIDER_DARK
    } else {
        theme::DIVIDER_LIGHT
    };
    let avail_w = ui.available_width();
    let (line_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 1.0), Sense::hover());
    ui.painter().line_segment(
        [line_rect.left_center(), line_rect.right_center()],
        Stroke::new(0.5, divider),
    );
    ui.add_space(theme::SPACE_XS);

    let Some(doc) = app.active_doc() else {
        ui.weak(app.i18n.t("sidebar-empty", &[]));
        return;
    };
    let w = doc.width();
    let h = doc.height();
    let total_px = (w as u64) * (h as u64);
    let est_mem_mb = (total_px * 4) as f64 / (1024.0 * 1024.0);

    egui::Grid::new("popover-props-grid")
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            prop_row(
                ui,
                app.i18n.t("prop-dimensions", &[]),
                format!("{w} × {h} px"),
            );
            prop_row(
                ui,
                app.i18n.t("prop-megapixels", &[]),
                format!("{:.2} MP", total_px as f64 / 1_000_000.0),
            );
            prop_row(
                ui,
                app.i18n.t("prop-aspect", &[]),
                aspect_ratio_string(w, h),
            );
            prop_row(
                ui,
                app.i18n.t("prop-channels", &[]),
                "RGBA 8-bit".to_string(),
            );
            prop_row(
                ui,
                app.i18n.t("prop-memory", &[]),
                format!("{est_mem_mb:.2} MB"),
            );

            if let Some(path) = &doc.path {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_uppercase();
                prop_row(
                    ui,
                    app.i18n.t("prop-format", &[]),
                    if ext.is_empty() { "—".to_string() } else { ext },
                );
                if let Ok(meta) = std::fs::metadata(path) {
                    prop_row(
                        ui,
                        app.i18n.t("prop-file-size", &[]),
                        format_bytes(meta.len()),
                    );
                    if let Ok(modified) = meta.modified() {
                        prop_row(
                            ui,
                            app.i18n.t("prop-modified", &[]),
                            format_time(modified),
                        );
                    }
                }
            }
        });

    if let Some(path) = &doc.path {
        ui.add_space(theme::SPACE_XS);
        ui.weak(
            egui::RichText::new(app.i18n.t("prop-path", &[])).size(theme::FONT_TINY),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(path.display().to_string())
                    .monospace()
                    .size(theme::FONT_TINY),
            )
            .wrap(),
        );
    }
}

fn prop_row(ui: &mut egui::Ui, label: String, value: String) {
    ui.label(
        egui::RichText::new(label)
            .size(theme::FONT_CAPTION)
            .color(Color32::from_gray(130)),
    );
    ui.label(egui::RichText::new(value).size(theme::FONT_CAPTION));
    ui.end_row();
}

fn text_secondary(app: &YImageApp) -> Color32 {
    if app.settings.theme_dark {
        theme::TEXT_SECONDARY_DARK
    } else {
        theme::TEXT_SECONDARY_LIGHT
    }
}

fn aspect_ratio_string(w: u32, h: u32) -> String {
    let gcd = {
        let mut a = w;
        let mut b = h;
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a.max(1)
    };
    format!(
        "{}:{} ({:.3})",
        w / gcd,
        h / gcd,
        w as f32 / h.max(1) as f32
    )
}

fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

fn format_time(t: std::time::SystemTime) -> String {
    match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs() as i64;
            let (y, mo, d, h, mi) = civil_from_unix(secs);
            format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
        }
        Err(_) => "—".into(),
    }
}

fn civil_from_unix(secs: i64) -> (i32, u8, u8, u8, u8) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let hour = (rem / 3600) as u8;
    let minute = ((rem % 3600) / 60) as u8;

    let z = days + 719468;
    let era = if z >= 0 {
        z / 146097
    } else {
        (z - 146096) / 146097
    };
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m, d, hour, minute)
}
