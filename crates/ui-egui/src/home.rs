//! Home: a quiet start page. Open a PDF, optional actions, pinned tools/folders, recent files,
//! and an honest developer/credit footer. Every section can be turned off from Customize.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use pdfcraft_engine::catalog;

use crate::theme::{self, Tokens};
use crate::{APP_NAME, PdfCraftApp, icons, panels::human_size, widgets};

pub mod folders_ui;

/// A soft shadow that rises on hover and settles when the card is pressed. Shared with the
/// All-tools catalog.
pub(crate) fn card_lift(ui: &egui::Ui, t: &Tokens, rect: Rect, press: theme::Press) {
    let lift = press.hover * (1.0 - press.down);
    if lift > 0.04 {
        let shadow = egui::Shadow { offset: [0, 3], blur: (14.0 * lift) as u8, spread: 0, color: t.page_shadow.gamma_multiply(0.85 * lift) };
        ui.painter().add(shadow.as_shape(rect, CornerRadius::same(10)));
    }
}

fn section_label(ui: &mut egui::Ui, text: &str, t: &Tokens) {
    ui.add_space(22.0);
    ui.label(egui::RichText::new(text).font(theme::semibold(10.5)).color(t.text_faint).extra_letter_spacing(1.0));
    ui.add_space(8.0);
}

/// Draw the Home page.
pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let column = ui.available_width().clamp(320.0, 1160.0);
        let side = ((ui.available_width() - column) / 2.0).max(24.0);
        ui.add_space(30.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(column);
                hero(app, ui, &t);
                if app.home_show_quick {
                    quick_actions(app, ui, &t);
                }
                pinned(app, ui, &t);
                recent(app, ui, &t);
                developer(app, ui, &t);
                ui.add_space(18.0);
                ui.label(egui::RichText::new(tl!("Local and private. No account, no telemetry.")).font(theme::regular(11.0)).color(t.text_faint));
                ui.add_space(28.0);
            });
            ui.add_space(side);
        });
    });
}

fn hero(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Image::from_bytes(
                "bytes://folio-icon.png",
                include_bytes!("../../../assets/app-icon/hicolor/128x128/apps/ai.storyteller.pdfcraft.png"),
            )
            .fit_to_exact_size(vec2(48.0, 48.0))
            .corner_radius(CornerRadius::same(11))
            .alt_text("Folio"),
        );
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(APP_NAME).font(theme::semibold(30.0)).color(t.text));
            ui.label(egui::RichText::new(tl!("A quieter place to read PDFs.")).font(theme::regular(13.5)).color(t.text_muted));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| customize(app, ui, t));
    });
    ui.add_space(24.0);

    let (rect, resp) = ui.allocate_exact_size(vec2(154.0, 42.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Open PDF")));
    let press = theme::Press::track(ui, &resp);
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        press.fill(t.accent, t.accent.gamma_multiply(0.9), t.accent.gamma_multiply(0.8)),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(body.left_center() + vec2(26.0, 0.0), vec2(19.0, 19.0)), "folder-open", 18.0, t.on_accent());
    ui.painter().text(body.left_center() + vec2(44.0, 0.0), Align2::LEFT_CENTER, tl!("Open PDF"), theme::semibold(13.5), t.on_accent());
    if theme::hand(resp).clicked() {
        app.open_dialog();
    }
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(tl!("or drop a PDF anywhere")).font(theme::regular(11.5)).color(t.text_faint));
    });
}

fn customize(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let resp = widgets::ghost_button(ui, "sliders-horizontal", tl!("Customize home"));
    egui::Popup::menu(&resp).id(resp.id.with("home_customize")).show(|ui| {
        ui.set_min_width(250.0);
        ui.label(egui::RichText::new(tl!("Sections on Home")).font(theme::semibold(13.0)).color(t.text));
        ui.add_space(6.0);
        ui.add(widgets::checkbox(&mut app.home_show_pinned, tl!("Pinned tools and folders")));
        ui.add(widgets::checkbox(&mut app.home_show_recent, tl!("Recent files")));
        ui.add(widgets::checkbox(&mut app.home_show_quick, tl!("Quick actions")));
        ui.add(widgets::checkbox(&mut app.home_show_developer, tl!("Developer and credits")));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let recent_label = ui.label(egui::RichText::new(tl!("Recent files to keep")).color(t.text_muted));
            let mut limit = app.recent_limit.min(crate::RECENT_FILES_MAX);
            if ui.add(egui::DragValue::new(&mut limit).range(0..=crate::RECENT_FILES_MAX)).labelled_by(recent_label.id).changed() {
                app.recent_limit = limit;
                app.recent.truncate(limit);
            }
        });
        ui.separator();
        if ui.button(tl!("Pin a tool…")).clicked() {
            app.dialog = Some(crate::Dialog::PinTool);
            ui.close();
        }
        if ui.button(tl!("Pin a folder…")).clicked() {
            app.execute("file.pin_folder");
            ui.close();
        }
    });
}

fn quick_actions(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    section_label(ui, tl!("Actions"), t);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(14.0, 8.0);
        if widgets::ghost_button(ui, "files", tl!("Combine files")).clicked() {
            app.execute("page.combine");
        }
        if widgets::ghost_button(ui, "layout-grid", tl!("Organize pages")).clicked() {
            app.execute("page.organize");
        }
        if widgets::ghost_button(ui, "pin", tl!("Pin a tool…")).clicked() {
            app.dialog = Some(crate::Dialog::PinTool);
        }
        if widgets::ghost_button(ui, "folder-plus", tl!("Pin a folder…")).clicked() {
            app.execute("file.pin_folder");
        }
    });
}

fn pinned(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    if !app.home_show_pinned || (app.pinned_tools.is_empty() && app.pinned.folders.is_empty() && app.pinned_files.is_empty()) {
        return;
    }
    ui.horizontal(|ui| {
        section_label(ui, tl!("Pinned"), t);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, "layout-grid", 24.0, app.pinned_grid, tl!("Grid view")).clicked() {
                app.pinned_grid = true;
            }
            if icons::button(ui, "list", 24.0, !app.pinned_grid, tl!("List view")).clicked() {
                app.pinned_grid = false;
            }
        });
    });

    if !app.pinned_tools.is_empty() {
        let pinned = app.pinned_tools.clone();
        let mut unpin = None;
        if app.pinned_grid {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for id in pinned {
                    let Some(g) = catalog::group(&id) else { continue };
                    let label = tl!(g.label);
                    let card_w = 126.0;
                    let card_h = 76.0;
                    let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                    let hovered = ui.rect_contains_pointer(rect);
                    let fill = if resp.hovered() { t.hover } else { t.card };
                    ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    icons::paint(
                        ui,
                        Rect::from_center_size(egui::pos2(rect.min.x + 24.0, rect.min.y + 24.0), vec2(22.0, 22.0)),
                        g.icon,
                        20.0,
                        t.accent,
                    );
                    widgets::row_text(ui, egui::pos2(rect.min.x + 12.0, rect.min.y + 50.0), label, theme::medium(11.5), t.text, card_w - 24.0);
                    if hovered {
                        let x_rect = Rect::from_center_size(rect.right_top() + vec2(-14.0, 14.0), vec2(20.0, 20.0));
                        let x = ui.interact(x_rect, ui.scope_id().with(("unpin-home-tool-grid", &id)), Sense::click());
                        if x.hovered() {
                            ui.painter().rect_filled(x_rect, CornerRadius::same(5), t.pressed);
                        }
                        icons::paint(ui, x_rect.shrink(4.0), "x", 12.0, t.icon);
                        if x.on_hover_text(tl!("Unpin from Home")).clicked() {
                            unpin = Some(g.id.to_string());
                        }
                    }
                    if resp.clicked() {
                        app.select_tool(g.id);
                    }
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for id in pinned {
                    let Some(g) = catalog::group(&id) else { continue };
                    let label = tl!(g.label);
                    let text_width = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), theme::medium(12.0), t.text).size().x);
                    let width = (text_width + 58.0).clamp(112.0, ui.available_width().max(112.0));
                    let (rect, resp) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                    let fill = if resp.hovered() { t.hover } else { t.card };
                    ui.painter().rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    icons::paint(
                        ui,
                        Rect::from_center_size(egui::pos2(rect.min.x + 18.0, rect.center().y), vec2(16.0, 16.0)),
                        g.icon,
                        15.0,
                        t.accent,
                    );
                    widgets::row_text(
                        ui,
                        egui::pos2(rect.min.x + 31.0, rect.center().y),
                        label,
                        theme::medium(11.5),
                        t.text,
                        (width - 52.0).max(1.0),
                    );
                    let x_rect = Rect::from_center_size(rect.right_center() - vec2(13.0, 0.0), vec2(22.0, 22.0));
                    let x = ui.interact(x_rect, ui.scope_id().with(("unpin-home-tool", &id)), Sense::click());
                    x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Unpin {label}")));
                    if x.hovered() {
                        ui.painter().rect_filled(x_rect, CornerRadius::same(6), t.pressed);
                    }
                    icons::paint(ui, x_rect.shrink(6.0), "x", 12.0, t.icon);
                    if x.on_hover_text(tl!("Unpin from Home")).clicked() {
                        unpin = Some(g.id.to_string());
                    } else if resp.clicked() {
                        app.select_tool(g.id);
                    }
                }
            });
        }
        if let Some(id) = unpin {
            app.unpin_tool(&id);
        }
    }
    if !app.pinned_files.is_empty() {
        ui.add_space(8.0);
        let pinned_files = app.pinned_files.clone();
        let mut unpin_file = None;
        let mut open_file = None;
        if app.pinned_grid {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for path in pinned_files {
                    let name = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).unwrap_or(&path);
                    let card_w = 120.0;
                    let card_h = 135.0;
                    let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
                    let press = theme::Press::track(ui, &resp);
                    let hovered = ui.rect_contains_pointer(rect);
                    let fill = press.fill(t.card, t.hover, t.pressed);
                    ui.painter().rect(rect, CornerRadius::same(8), fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    let preview = Rect::from_min_size(rect.min + vec2(12.0, 10.0), vec2(card_w - 24.0, 80.0));
                    ui.painter().rect_filled(preview, CornerRadius::same(4), t.pasteboard);
                    icons::paint(ui, Rect::from_center_size(preview.center(), vec2(28.0, 28.0)), "file-text", 28.0, t.accent);

                    if hovered {
                        let x_rect = Rect::from_center_size(rect.right_top() + vec2(-12.0, 12.0), vec2(20.0, 20.0));
                        let x = ui.interact(x_rect, ui.scope_id().with(("unpin-file-grid", &path)), Sense::click());
                        if x.hovered() {
                            ui.painter().rect_filled(x_rect, CornerRadius::same(4), t.pressed);
                        }
                        icons::paint(ui, x_rect.shrink(4.0), "x", 12.0, t.icon);
                        if x.on_hover_text(tl!("Unpin from Home")).clicked() {
                            unpin_file = Some(path.clone());
                        }
                    }

                    widgets::row_text(ui, egui::pos2(rect.min.x + 8.0, rect.min.y + 104.0), name, theme::medium(11.5), t.text, card_w - 16.0);

                    if theme::hand(resp).on_hover_text(&path).clicked() {
                        open_file = Some(path.clone());
                    }
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for path in pinned_files {
                    let name = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).unwrap_or(&path);
                    let text_width = ui.fonts_mut(|f| f.layout_no_wrap(name.to_owned(), theme::medium(12.0), t.text).size().x);
                    let width = (text_width + 58.0).clamp(120.0, ui.available_width().max(120.0));
                    let (rect, resp) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
                    let fill = if resp.hovered() { t.hover } else { t.card };
                    ui.painter().rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    icons::paint(
                        ui,
                        Rect::from_center_size(egui::pos2(rect.min.x + 18.0, rect.center().y), vec2(16.0, 16.0)),
                        "file-text",
                        15.0,
                        t.accent,
                    );
                    widgets::row_text(ui, egui::pos2(rect.min.x + 31.0, rect.center().y), name, theme::medium(11.5), t.text, (width - 52.0).max(1.0));
                    let x_rect = Rect::from_center_size(rect.right_center() - vec2(13.0, 0.0), vec2(22.0, 22.0));
                    let x = ui.interact(x_rect, ui.scope_id().with(("unpin-home-file", &path)), Sense::click());
                    if x.hovered() {
                        ui.painter().rect_filled(x_rect, CornerRadius::same(6), t.pressed);
                    }
                    icons::paint(ui, x_rect.shrink(6.0), "x", 12.0, t.icon);
                    if x.on_hover_text(tl!("Unpin from Home")).clicked() {
                        unpin_file = Some(path.clone());
                    } else if resp.clicked() {
                        open_file = Some(path.clone());
                    }
                }
            });
        }
        if let Some(path) = unpin_file {
            app.unpin_file(&path);
        }
        if let Some(path) = open_file {
            app.open_recent(&path);
        }
    }
    if !app.pinned.folders.is_empty() {
        crate::folders_ui::section(app, ui);
    }
}

fn recent_incognito_card(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        let (icon_rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
        ui.painter().circle_filled(icon_rect.center(), 22.0, t.card);
        icons::paint(ui, icon_rect.shrink(10.0), "eye-off", 22.0, t.text_muted);
        ui.add_space(8.0);
        ui.label(egui::RichText::new(tl!("You've gone Incognito")).font(theme::semibold(14.0)).color(t.text));
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(tl!("The history of the files you have viewed will not be saved.")).font(theme::regular(12.0)).color(t.text_muted),
        );
        ui.add_space(12.0);
        if ui.add(egui::Button::new(tl!("Close")).min_size(vec2(80.0, 28.0))).clicked() {
            app.incognito = false;
        }
        ui.add_space(24.0);
    });
}

fn recent_grid_view(
    app: &PdfCraftApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    open: &mut Option<String>,
    remove: &mut Option<String>,
    toggle_pin: &mut Option<String>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
        for r in app.recent.iter().take(app.recent_limit) {
            let card_w = 120.0;
            let card_h = 135.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &r.name));
            let press = theme::Press::track(ui, &resp);
            let hovered = ui.rect_contains_pointer(rect);
            let fill = press.fill(t.card, t.hover, t.pressed);
            ui.painter().rect(rect, CornerRadius::same(8), fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            let preview = Rect::from_min_size(rect.min + vec2(12.0, 10.0), vec2(card_w - 24.0, 80.0));
            ui.painter().rect_filled(preview, CornerRadius::same(4), t.pasteboard);
            icons::paint(ui, Rect::from_center_size(preview.center(), vec2(28.0, 28.0)), "file-text", 28.0, t.icon);

            if hovered || app.is_file_pinned(&r.path) {
                let is_pinned = app.is_file_pinned(&r.path);
                let pin_rect = Rect::from_center_size(rect.right_top() + vec2(-34.0, 12.0), vec2(20.0, 20.0));
                let pin_btn = ui.interact(pin_rect, ui.scope_id().with(("pin-recent-grid", &r.path)), Sense::click());
                if pin_btn.hovered() {
                    ui.painter().rect_filled(pin_rect, CornerRadius::same(4), t.pressed);
                }
                icons::paint(ui, pin_rect.shrink(4.0), "pin", 12.0, if is_pinned { t.accent } else { t.icon });
                let tip = if is_pinned { tl!("Unpin from Home") } else { tl!("Pin to Home") };
                if pin_btn.on_hover_text(tip).clicked() {
                    *toggle_pin = Some(r.path.clone());
                }

                let x_rect = Rect::from_center_size(rect.right_top() + vec2(-12.0, 12.0), vec2(20.0, 20.0));
                let x = ui.interact(x_rect, ui.scope_id().with(("remove-recent-grid", &r.path)), Sense::click());
                if x.hovered() {
                    ui.painter().rect_filled(x_rect, CornerRadius::same(4), t.pressed);
                }
                icons::paint(ui, x_rect.shrink(4.0), "x", 12.0, t.icon);
                if x.on_hover_text(tl!("Remove from Recent")).clicked() {
                    *remove = Some(r.path.clone());
                }
            }

            widgets::row_text(ui, egui::pos2(rect.min.x + 8.0, rect.min.y + 104.0), r.name.as_str(), theme::medium(11.5), t.text, card_w - 16.0);
            let pages = format!("{}  ·  {}", crate::i18n::trn(crate::i18n::current(), r.pages as u64, "{n} page", "{n} pages"), human_size(r.size));
            widgets::row_text(ui, egui::pos2(rect.min.x + 8.0, rect.min.y + 120.0), pages, theme::regular(10.0), t.text_muted, card_w - 16.0);

            if theme::hand(resp).on_hover_text(&r.path).clicked() {
                *open = Some(r.path.clone());
            }
        }
    });
}

fn recent_list_view(
    app: &PdfCraftApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    open: &mut Option<String>,
    remove: &mut Option<String>,
    toggle_pin: &mut Option<String>,
) {
    for r in app.recent.iter().take(app.recent_limit) {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &r.name));
        let press = theme::Press::track(ui, &resp);
        let hovered = ui.rect_contains_pointer(rect);
        press.wash(ui, rect, 8, false);
        if hovered || app.is_file_pinned(&r.path) {
            let is_pinned = app.is_file_pinned(&r.path);
            let pin_rect = Rect::from_center_size(rect.right_center() - vec2(58.0, 0.0), vec2(28.0, 28.0));
            let pin_btn = ui.interact(pin_rect, ui.scope_id().with(("pin-recent", &r.path)), Sense::click());
            if pin_btn.hovered() {
                ui.painter().rect_filled(pin_rect, CornerRadius::same(6), t.pressed);
            }
            icons::paint(ui, pin_rect.shrink(6.0), "pin", 16.0, if is_pinned { t.accent } else { t.icon });
            let tip = if is_pinned { tl!("Unpin from Home") } else { tl!("Pin to Home") };
            if pin_btn.on_hover_text(tip).clicked() {
                *toggle_pin = Some(r.path.clone());
            }

            let x_rect = Rect::from_center_size(rect.right_center() - vec2(26.0, 0.0), vec2(28.0, 28.0));
            let x = ui.interact(x_rect, ui.scope_id().with(("remove-recent", &r.path)), Sense::click());
            let label = crate::i18n::fmt(tl!("Remove {name} from Recent"), &[("name", &r.name)]);
            x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            if x.hovered() {
                ui.painter().rect_filled(x_rect, CornerRadius::same(6), t.pressed);
            }
            icons::paint(ui, x_rect.shrink(6.0), "x", 16.0, t.icon);
            if x.on_hover_text(tl!("Remove from Recent")).clicked() {
                *remove = Some(r.path.clone());
            }
        }
        let body = rect.translate(press.offset());
        icons::paint(ui, Rect::from_min_size(body.min + vec2(10.0, 12.0), vec2(22.0, 22.0)), "file-text", 20.0, t.icon);
        let detail = ui.painter().text(
            body.right_center() - vec2(78.0, 0.0),
            Align2::RIGHT_CENTER,
            format!("{}  ·  {}", crate::i18n::trn(crate::i18n::current(), r.pages as u64, "{n} page", "{n} pages"), human_size(r.size)),
            theme::regular(11.5),
            t.text_muted,
        );
        let width = detail.left() - 16.0 - (body.left() + 44.0);
        widgets::row_text(ui, body.min + vec2(44.0, 16.0), r.name.as_str(), theme::medium(13.0), t.text, width);
        widgets::row_text(ui, body.min + vec2(44.0, 33.0), r.path.as_str(), theme::regular(10.8), t.text_faint, width);
        if theme::hand(resp).on_hover_text(&r.path).clicked() {
            *open = Some(r.path.clone());
        }
    }
}

fn recent(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    if !app.home_show_recent || app.recent_limit == 0 {
        return;
    }
    let mut clear = false;
    ui.horizontal(|ui| {
        section_label(ui, tl!("Recent"), t);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::ghost_button(ui, "trash-2", tl!("Clear")).clicked() {
                clear = true;
            }
            if icons::button(ui, "layout-grid", 24.0, app.recent_grid, tl!("Grid view")).clicked() {
                app.recent_grid = true;
            }
            if icons::button(ui, "list", 24.0, !app.recent_grid, tl!("List view")).clicked() {
                app.recent_grid = false;
            }
            if icons::button(ui, "eye-off", 24.0, app.incognito, tl!("Incognito mode")).clicked() {
                app.incognito = !app.incognito;
            }
        });
    });

    if app.incognito {
        recent_incognito_card(app, ui, t);
        return;
    }

    if app.recent.is_empty() {
        ui.label(
            egui::RichText::new(format!("Files you open in {APP_NAME} appear here. Drop a PDF anywhere to open it."))
                .font(theme::regular(12.5))
                .color(t.text_muted),
        );
        return;
    }
    let mut open = None;
    let mut remove = None;
    let mut toggle_pin = None;
    if app.recent_grid {
        recent_grid_view(app, ui, t, &mut open, &mut remove, &mut toggle_pin);
    } else {
        recent_list_view(app, ui, t, &mut open, &mut remove, &mut toggle_pin);
    }
    if let Some(p) = open {
        app.open_recent(&p);
    }
    if let Some(p) = remove {
        app.recent.retain(|r| r.path != p);
    }
    if let Some(p) = toggle_pin {
        app.toggle_pin_file(&p);
    }
    if clear {
        app.execute("file.clear_recent");
    }
}

fn developer(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    if !app.home_show_developer {
        return;
    }
    ui.add_space(26.0);
    ui.separator();
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::Image::from_bytes("bytes://tinzur-qr.png", include_bytes!("../../../assets/brand/tinzur-qr.png"))
                .fit_to_exact_size(vec2(72.0, 76.0))
                .corner_radius(CornerRadius::same(8))
                .alt_text("Telegram QR code for @TINZUR"),
        );
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(tl!("Built on PdfCraft")).font(theme::semibold(13.0)).color(t.text));
            ui.label(
                egui::RichText::new(format!(
                    "{} is an independent, distraction-free reading shell by TINZUR, built on the open-source PdfCraft project.",
                    APP_NAME
                ))
                .font(theme::regular(12.0))
                .color(t.text_muted),
            );
            ui.label(
                egui::RichText::new(tl!(
                    "The PDF engine and file handling are PdfCraft's; the monochrome theme, two-row tools and split view are this redesign."
                ))
                .font(theme::regular(11.0))
                .color(t.text_faint),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if widgets::ghost_button(ui, "send", tl!("Telegram — @TINZUR")).clicked() {
                    app.open_url(pdfcraft_engine::links::TELEGRAM);
                }
                if widgets::ghost_button(ui, "code-xml", tl!("PdfCraft source")).clicked() {
                    app.open_url(pdfcraft_engine::links::GITHUB);
                }
            });
        });
    });
}
