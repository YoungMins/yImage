// Top chrome: two rows.
//
// Row 1 — Tab bar: [≡ Menu] [Tab1] [Tab2] [Tab3 ×] ...
// Row 2 — Ribbon toolbar: grouped tool sections with labels.
//
// The tab bar always occupies the topmost strip. The ribbon sits directly
// below it and organises tools into named groups separated by thin vertical
// hairlines, similar to a simplified Office-style ribbon.

use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Vec2};

use crate::app::YImageApp;
#[cfg(windows)]
use crate::app::BgMsg;
use crate::tools::ToolKind;
use crate::ui::theme;

const TAB_BAR_HEIGHT: f32 = 38.0;
const RIBBON_HEIGHT: f32 = 56.0;
const TOOL_BTN_HEIGHT: f32 = 34.0;

// ── Row 1: Tab bar ─────────────────────────────────────────────────

pub fn show_tab_bar(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::top("tab_bar")
        .exact_height(TAB_BAR_HEIGHT)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(4.0);

                // Hamburger menu — far left.
                menu_button(ctx, ui, app);
                ui.add_space(6.0);

                // Tabs.
                tabs_zone(ui, app);
            });
        });
}

// ── Row 2: Ribbon toolbar ──────────────────────────────────────────

pub fn show_ribbon(ctx: &egui::Context, app: &mut YImageApp) {
    egui::TopBottomPanel::top("ribbon_toolbar")
        .exact_height(RIBBON_HEIGHT)
        .show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(8.0);

                // ── Selection group ──
                ribbon_group(ui, app.settings.theme_dark, &app.i18n.t("tool-none", &[]), |ui| {
                    tool_btn(ui, app, ToolKind::None, "\u{2196}", "tool-none");
                });

                group_separator(ui);

                // ── Drawing group ──
                ribbon_group(ui, app.settings.theme_dark, &app.i18n.t("menu-edit", &[]), |ui| {
                    tool_btn(ui, app, ToolKind::Draw, "\u{270F}", "tool-draw");
                    tool_btn(ui, app, ToolKind::Mosaic, "\u{25A3}", "tool-mosaic");
                    tool_btn(ui, app, ToolKind::Text, "T", "tool-text");
                    tool_btn(ui, app, ToolKind::Shape, "\u{25FB}", "tool-shape");
                });

                group_separator(ui);

                // ── AI group ──
                ribbon_group(ui, app.settings.theme_dark, "AI", |ui| {
                    tool_btn(
                        ui,
                        app,
                        ToolKind::BackgroundRemove,
                        "\u{2702}",
                        "tool-bg-remove",
                    );
                    tool_btn(
                        ui,
                        app,
                        ToolKind::ObjectRemove,
                        "\u{2296}",
                        "tool-obj-remove",
                    );
                });

                group_separator(ui);

                // ── Animation group ──
                ribbon_group(ui, app.settings.theme_dark, "GIF", |ui| {
                    tool_btn(ui, app, ToolKind::Gif, "\u{25B6}", "tool-gif");
                });
            });
        });
}

/// Draw a named ribbon group: a vertical stack of [tools row] + [label].
fn ribbon_group(
    ui: &mut egui::Ui,
    dark: bool,
    section_label: &str,
    tools: impl FnOnce(&mut egui::Ui),
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            tools(ui);
        });
        ui.add_space(1.0);
        let text_color = if dark {
            theme::TEXT_SECONDARY_DARK
        } else {
            theme::TEXT_SECONDARY_LIGHT
        };
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(section_label)
                    .size(theme::FONT_TINY)
                    .color(text_color),
            );
        });
    });
}

fn group_separator(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, RIBBON_HEIGHT - 12.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    ui.add_space(4.0);
}

// ── Tabs ───────────────────────────────────────────────────────────

fn tabs_zone(ui: &mut egui::Ui, app: &mut YImageApp) {
    let mut switch_to: Option<usize> = None;
    let mut close_idx: Option<usize> = None;

    for (i, tab) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;
        let title = tab.title();
        let dirty_marker = if tab.doc.dirty { "\u{25CF} " } else { "" };

        let fill = if is_active {
            theme::ACCENT.linear_multiply(0.14)
        } else {
            Color32::TRANSPARENT
        };
        let text_color = if is_active {
            theme::ACCENT
        } else {
            ui.visuals().text_color()
        };

        // Tab pill as a single clickable region.
        let frame = egui::Frame::none()
            .inner_margin(Margin::symmetric(10, 4))
            .corner_radius(CornerRadius::same(8))
            .fill(fill);

        let tab_id = ui.id().with(("tab_hover", i));
        let inner = frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // Tab label — clicking switches tab.
                let label_resp = ui.add(
                    egui::Label::new(
                        RichText::new(format!("{dirty_marker}{title}"))
                            .size(12.5)
                            .color(text_color),
                    )
                    .sense(egui::Sense::click()),
                );
                if label_resp.clicked() {
                    switch_to = Some(i);
                }

                // Close button fades in when the tab is hovered or active, so
                // resting tabs stay visually quiet.
                let hovered = ui
                    .ctx()
                    .animate_bool_with_time(tab_id, false, 0.12);
                let opacity = if is_active { 1.0 } else { hovered.max(0.15) };
                let base = ui.visuals().text_color();
                let x_resp = ui.add(
                    egui::Button::new(
                        RichText::new("\u{00D7}")
                            .size(13.0)
                            .color(base.linear_multiply(0.35 + 0.4 * opacity)),
                    )
                    .frame(false)
                    .min_size(Vec2::new(18.0, 18.0)),
                );
                if x_resp.clicked() {
                    close_idx = Some(i);
                }
            });
        });

        let any_hovered = inner.response.hovered()
            || inner.response.contains_pointer();
        ui.ctx().animate_bool_with_time(tab_id, any_hovered, 0.12);
    }

    if let Some(idx) = close_idx {
        app.close_tab(idx);
    } else if let Some(idx) = switch_to {
        if idx != app.active_tab {
            app.dialog.obj_mask = None;
        }
        app.active_tab = idx;
    }
}

// ── Tool button ────────────────────────────────────────────────────

fn tool_btn(
    ui: &mut egui::Ui,
    app: &mut YImageApp,
    kind: ToolKind,
    icon: &str,
    label_key: &str,
) {
    let is_active = app.tool == kind;
    let label = app.i18n.t(label_key, &[]);

    let fill = if is_active {
        theme::ACCENT
    } else {
        Color32::TRANSPARENT
    };
    let text_color = if is_active {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };

    let btn = egui::Button::new(
        RichText::new(format!("{icon} {label}"))
            .size(12.0)
            .color(text_color),
    )
    .min_size(Vec2::new(0.0, TOOL_BTN_HEIGHT))
    .fill(fill)
    .corner_radius(CornerRadius::same(8));

    let r = ui.add(btn).on_hover_text(&label);
    if r.clicked() {
        app.tool = kind;
    }
}

// ── Menu ───────────────────────────────────────────────────────────

/// Render a menu row: primary label on the left, right-aligned keyboard
/// shortcut hint rendered in a dim monospace. Returns the button's Response
/// so callers can check `.clicked()`.
fn menu_row(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
    shortcut: &str,
) -> egui::Response {
    let row = ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            ui.set_min_width(220.0);
            let resp = ui.add(
                egui::Button::new(RichText::new(label).size(theme::FONT_BODY))
                    .frame(false)
                    .min_size(Vec2::new(160.0, 22.0)),
            );
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let color = ui.visuals().text_color().linear_multiply(0.45);
                    ui.label(
                        RichText::new(shortcut)
                            .size(theme::FONT_TINY)
                            .monospace()
                            .color(color),
                    );
                },
            );
            resp
        })
        .inner
    });
    row.inner
}

fn menu_button(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    let menu_resp = ui.menu_button(
        RichText::new("\u{2630}").size(16.0),
        |ui| {
            ui.set_min_width(220.0);
            file_section(ctx, ui, app);
            ui.separator();
            edit_section(ui, app);
            ui.separator();
            view_section(ctx, ui, app);
            ui.separator();
            language_section(ui, app);
        },
    );
    menu_resp
        .response
        .on_hover_text(app.i18n.t("header-menu", &[]));
}

fn file_section(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    if menu_row(ui, true, &format!("\u{1F4C2}  {}", app.i18n.t("action-open", &[])), "Ctrl+O")
        .clicked()
    {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter(
                "images",
                &[
                    "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "avif",
                ],
            )
            .pick_file()
        {
            app.open_path(&p, true);
        }
        ui.close_menu();
    }
    if menu_row(
        ui,
        app.has_doc(),
        &format!("\u{1F4BE}  {}", app.i18n.t("action-save", &[])),
        "Ctrl+S",
    )
    .clicked()
    {
        app.save_current();
        ui.close_menu();
    }
    if menu_row(
        ui,
        app.has_doc(),
        &format!("\u{1F4BE}  {}", app.i18n.t("action-save-as", &[])),
        "Ctrl+Shift+S",
    )
    .clicked()
    {
        app.dialog.save_dialog_open = true;
        ui.close_menu();
    }

    ui.separator();
    if ui
        .add_enabled(
            app.has_doc(),
            egui::Button::new(format!(
                "\u{26A1}  {}",
                app.i18n.t("action-optimize", &[])
            )),
        )
        .clicked()
    {
        app.dialog.optimize_open = true;
        ui.close_menu();
    }
    if ui
        .add_enabled(
            app.has_doc(),
            egui::Button::new(format!(
                "\u{2194}  {}",
                app.i18n.t("action-resize", &[])
            )),
        )
        .clicked()
    {
        app.dialog.resize_open = true;
        if let Some(tab) = app.tabs.get(app.active_tab) {
            app.dialog.resize_w = tab.doc.width();
            app.dialog.resize_h = tab.doc.height();
        }
        ui.close_menu();
    }
    if ui
        .add_enabled(
            app.has_doc(),
            egui::Button::new(format!(
                "\u{21C4}  {}",
                app.i18n.t("action-convert", &[])
            )),
        )
        .clicked()
    {
        app.dialog.convert_open = true;
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button(format!("\u{1F39E}  {}", app.i18n.t("action-gif", &[])))
        .clicked()
    {
        app.tool = ToolKind::Gif;
        app.dialog.gif_timeline_open = true;
        ui.close_menu();
    }

    #[cfg(all(windows, feature = "capture"))]
    capture_menu(ui, app);

    #[cfg(windows)]
    {
        ui.separator();
        if ui
            .button(format!(
                "\u{2B50}  {}",
                app.i18n.t("action-set-default", &[])
            ))
            .clicked()
        {
            match crate::registry::register_file_associations() {
                Ok(_) => {
                    let _ = app
                        .tx
                        .send(BgMsg::Info(app.i18n.t("status-default-ok", &[])));
                }
                Err(e) => {
                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
            ui.close_menu();
        }
        if ui
            .button(format!(
                "\u{2795}  {}",
                app.i18n.t("action-register-context", &[])
            ))
            .clicked()
        {
            let labels = crate::registry::ContextMenuLabels {
                root: app.i18n.t("ctx-root", &[]),
                open: app.i18n.t("ctx-open", &[]),
                optimize: app.i18n.t("ctx-optimize", &[]),
                resize: app.i18n.t("ctx-resize", &[]),
                convert: app.i18n.t("ctx-convert", &[]),
                bg_remove: app.i18n.t("ctx-bg-remove", &[]),
                obj_remove: app.i18n.t("ctx-obj-remove", &[]),
            };
            match crate::registry::register_context_menu(&labels) {
                Ok(_) => {
                    let _ = app
                        .tx
                        .send(BgMsg::Info(app.i18n.t("status-context-ok", &[])));
                }
                Err(e) => {
                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
            ui.close_menu();
        }
        if ui
            .button(format!(
                "\u{2796}  {}",
                app.i18n.t("action-unregister-context", &[])
            ))
            .clicked()
        {
            match crate::registry::unregister_context_menu() {
                Ok(_) => {
                    let _ = app
                        .tx
                        .send(BgMsg::Info(app.i18n.t("status-context-removed", &[])));
                }
                Err(e) => {
                    let _ = app.tx.send(BgMsg::Error(format!("{e:#}")));
                }
            }
            ui.close_menu();
        }
    }
    ui.separator();
    if ui
        .button(format!("\u{23FB}  {}", app.i18n.t("action-quit", &[])))
        .clicked()
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn edit_section(ui: &mut egui::Ui, app: &mut YImageApp) {
    if menu_row(ui, true, &app.i18n.t("action-undo", &[]), "Ctrl+Z").clicked() {
        if let Some(tab) = app.active_tab_mut() {
            if tab.doc.undo() {
                tab.texture_dirty = true;
            }
        }
        ui.close_menu();
    }
    if menu_row(ui, true, &app.i18n.t("action-redo", &[]), "Ctrl+Shift+Z").clicked() {
        if let Some(tab) = app.active_tab_mut() {
            if tab.doc.redo() {
                tab.texture_dirty = true;
            }
        }
        ui.close_menu();
    }
}

fn view_section(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut YImageApp) {
    if ui
        .button(app.i18n.t("action-fit", &[]))
        .clicked()
    {
        if let Some(tab) = app.active_tab_mut() {
            tab.viewer.reset_view = true;
        }
        ui.close_menu();
    }
    if ui
        .button(app.i18n.t("action-zoom-100", &[]))
        .clicked()
    {
        if let Some(tab) = app.active_tab_mut() {
            tab.viewer.zoom = 1.0;
        }
        ui.close_menu();
    }
    ui.separator();
    let mut show_thumbs = app.thumbs.visible;
    if ui
        .checkbox(&mut show_thumbs, app.i18n.t("view-thumbnails", &[]))
        .changed()
    {
        app.thumbs.visible = show_thumbs;
    }
    ui.separator();
    let mut dark = app.settings.theme_dark;
    if ui
        .checkbox(&mut dark, app.i18n.t("view-dark-theme", &[]))
        .changed()
    {
        app.settings.theme_dark = dark;
        if dark {
            theme::apply_dark(ctx);
        } else {
            theme::apply_light(ctx);
        }
    }
    #[cfg(all(windows, feature = "capture"))]
    {
        ui.separator();
        if ui
            .button(app.i18n.t("menu-hotkeys", &[]))
            .clicked()
        {
            app.dialog.hotkeys_open = true;
            ui.close_menu();
        }
    }
}

fn language_section(ui: &mut egui::Ui, app: &mut YImageApp) {
    ui.menu_button(app.i18n.t("menu-lang", &[]), |ui| {
        for lang in ["en-US", "ko-KR", "ja-JP"] {
            if ui.button(lang).clicked() {
                app.settings.language = lang.to_string();
                app.i18n = crate::i18n::I18n::new(lang);
                ui.close_menu();
            }
        }
    });
    ui.hyperlink_to("\u{2764} Ko-fi", "https://ko-fi.com/youngminkim");
}

#[cfg(all(windows, feature = "capture"))]
fn capture_menu(ui: &mut egui::Ui, app: &mut YImageApp) {
    let title = format!("\u{1F4F7}  {}", app.i18n.t("action-capture", &[]));
    ui.menu_button(title, |ui| {
        if ui
            .button(format!("\u{25A3}  {}", app.i18n.t("cap-fullscreen", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::Fullscreen);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F1}  {}", app.i18n.t("cap-window", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::ActiveWindow);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F0}  {}", app.i18n.t("cap-region", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::Region);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{25F3}  {}", app.i18n.t("cap-fixed", &[])))
            .clicked()
        {
            let mode = match app.dialog.fixed_region {
                Some((x, y, w, h)) => crate::capture::CaptureMode::FixedRegion { x, y, w, h },
                None => crate::capture::CaptureMode::FixedRegion {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            };
            app.trigger_capture(mode);
            ui.close_menu();
        }
        if ui
            .button(format!("\u{21C5}  {}", app.i18n.t("cap-scroll", &[])))
            .clicked()
        {
            app.trigger_capture(crate::capture::CaptureMode::AutoScroll);
            ui.close_menu();
        }
    });
}
