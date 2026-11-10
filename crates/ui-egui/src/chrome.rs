//! Window chrome: tab strip (with the integrated macOS title bar), mode bar, right rail.

use egui::{Align, Color32, CornerRadius, Layout, Rect, Sense, Stroke, vec2};

use crate::theme::{self, ThemePreference, Tokens};
use crate::{Dialog, PdfCraftApp, PropsTab, icons, widgets, window_controls};

/// Height of the tab strip when it doubles as the macOS title bar (#789). AppKit draws the traffic
/// lights itself, centred in the standard 28 pt title-bar band at the top of the window, and moving
/// them takes AppKit calls the workspace forbids (`unsafe`). So on macOS the strip is that band:
/// the lights sit vertically centred in it, and the controls beside them shrink to fit, as in
/// WordCraft (storytold/wordcraft#255).
pub const MAC_TITLE_BAR: f32 = 28.0;
/// The traffic lights as AppKit lays them out: button diameter, centre-to-centre pitch and the
/// first button's left edge (8–10 pt depending on the macOS version; the larger value keeps the
/// gap after them from shrinking).
const MAC_LIGHT: f32 = 12.0;
const MAC_LIGHT_PITCH: f32 = 20.0;
const MAC_LIGHTS_LEFT: f32 = 10.0;
/// Where the strip's content starts to the right of the traffic lights: the end of the three
/// lights plus the same gap the lights keep from the top of the window.
pub const MAC_CONTENT_LEFT: f32 = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT + (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;

/// Draw the window's tab strip: home and open controls, shrinkable and scrollable document tabs with drag
/// reordering, and platform window controls.
pub fn tab_strip(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    // macOS: the strip is the 28 pt title bar, so the traffic lights are centred in it; tabs and
    // buttons shrink to keep a margin above and below.
    let mac = cfg!(target_os = "macos") && app.integrated_titlebar;
    let left = if mac { MAC_CONTENT_LEFT as i8 } else { 8 };
    let default_h = if mac { MAC_TITLE_BAR } else { 38.0 };
    let base_h = if app.top_bar_size > 0.0 { app.top_bar_size } else { default_h }.clamp(24.0, 48.0);
    let (tab_h, button) =
        if mac { (base_h - 2.0, (base_h - 4.0).clamp(18.0, 40.0)) } else { ((base_h - 8.0).clamp(24.0, 44.0), (base_h - 10.0).clamp(20.0, 40.0)) };
    egui::Panel::top("tab_strip")
        .exact_size(base_h)
        .frame(egui::Frame::NONE.fill(t.titlebar).inner_margin(egui::Margin { left, right: 10, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            let drag = ui.interact(full, ui.scope_id().with("titledrag"), Sense::click_and_drag());
            if drag.drag_started() {
                window_controls::desktop_grab(ui.ctx(), egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
            // GNOME on Wayland: the strip is the header bar, with the window buttons at its ends.
            let fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
            let controls = app
                .window_controls
                .clone()
                .filter(|_| app.draws_window_frame() && !fullscreen)
                .unwrap_or_else(|| window_controls::Layout { left: Vec::new(), right: Vec::new() });
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                // The in-window menu bar where the desktop has none (the macOS menu bar is
                // native). It shares the document row, so removing the mode bar cost no menus.
                if app.native_menu.is_none() {
                    app.menu_bar_ui(ui);
                    ui.add_space(4.0);
                    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(8.0), Stroke::new(1.0, t.divider));
                    ui.add_space(6.0);
                }
                if !controls.left.is_empty() {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        window_controls::buttons(ui, &controls.left);
                    });
                    ui.add_space(window_controls::SIDE_GAP - 4.0);
                }
                if icons::button(ui, "house", button, app.active.is_none() && !app.combine_showing(), tl!("Home")).clicked() {
                    app.active = None;
                    app.combine_tab.focused = false;
                }
                if icons::button(ui, "circle-help", button, false, tl!("Keyboard shortcuts")).clicked() {
                    app.dialog = Some(Dialog::Shortcuts);
                }
                let (theme_icon, theme_label) = match app.theme_preference {
                    ThemePreference::System => ("monitor", tl!("Use system setting")),
                    ThemePreference::Light => ("sun", tl!("Light gray")),
                    ThemePreference::Dark => ("moon", tl!("Dark gray")),
                    ThemePreference::Sepia => ("book-open", tl!("Sepia")),
                    ThemePreference::Nordic => ("snowflake", tl!("Nordic")),
                    ThemePreference::Dracula => ("sparkles", tl!("Dracula")),
                };
                let theme_tip = format!("{}: {theme_label}", tl!("Display theme"));
                let theme_resp = icons::button(ui, theme_icon, button, false, &theme_tip);
                if theme_resp.clicked() {
                    app.execute(match app.theme_preference {
                        ThemePreference::System => "view.theme.light",
                        ThemePreference::Light => "view.theme.dark",
                        ThemePreference::Dark => "view.theme.sepia",
                        ThemePreference::Sepia => "view.theme.nordic",
                        ThemePreference::Nordic => "view.theme.dracula",
                        ThemePreference::Dracula => "view.theme.system",
                    });
                }
                egui::Popup::context_menu(&theme_resp).id(theme_resp.id.with("theme_menu")).show(|ui| appearance_menu(app, ui));
                if icons::button(ui, "eye", button, app.focus_mode, tl!("Focus mode")).clicked() {
                    app.focus_mode = !app.focus_mode;
                }
                // Reserve the four document actions and the window controls outside the scrolling
                // tabs, and the three app icons that follow Home (shortcuts, theme, focus).
                let controls_width = 4.0 * button + 16.0 + window_controls::width(&controls.right);
                let app_icons = 3.0 * button + 12.0;
                // Open stays in sight after the tabs: its icon, text and padding, and the gaps around it.
                let open_width = ui.fonts_mut(|f| f.layout_no_wrap(tl!("Open").into(), theme::medium(13.0), t.text).size().x) + 38.0 + 12.0;
                let mut tabs_width = (ui.available_width() - controls_width - open_width - app_icons).max(0.0);
                // Tabs shrink to share the strip (names end in "…") and scroll only once they reach
                // their narrowest; then arrows either side say so and step through them.
                let mut natural: Vec<f32> = Vec::with_capacity(app.views.len() + 1);
                for i in 0..app.views.len() {
                    if let Some(name) = crate::canvas::tab_label(app, i) {
                        natural.push(tab_natural_width(ui, &t, &name));
                    }
                }
                if app.combine_tab.open {
                    natural.push(tab_natural_width(ui, &t, tl!("Combine files")));
                }
                let gap = ui.spacing().item_spacing.x;
                let overflow = tabs_overflow(&natural, tabs_width, gap, TAB_MIN_WIDTH);
                if overflow {
                    tabs_width = (tabs_width - 2.0 * (TAB_ARROW + gap)).max(0.0);
                }
                let cap = tab_cap(&natural, tabs_width, gap, TAB_MIN_WIDTH);
                // Where the tabs were scrolled to last frame, and how far they can go.
                let scroll_id = ui.scope_id().with("tab_scroll");
                let (offset, max_offset) = ui.data(|d| d.get_temp::<(f32, f32)>(scroll_id)).unwrap_or((0.0, 0.0));
                let mut scroll_to = None;
                if overflow && arrow(ui, "chevron-left", offset > 0.5, tl!("Scroll tabs left")) {
                    scroll_to = Some((offset - (TAB_MIN_WIDTH + gap)).max(0.0));
                }
                // Keyed by document, not index: a tab dragged past the active one moves it without
                // the strip jumping to it.
                let state = (app.active_ids().map(|(_, id)| id), app.views.len(), app.combine_showing());
                let changed = ui.data_mut(|data| {
                    let id = ui.scope_id().with("active_tab");
                    let changed = data.get_temp::<(Option<pdfcraft_engine::DocId>, usize, bool)>(id) != Some(state);
                    data.insert_temp(id, state);
                    changed
                });
                let mut action = None;
                let mut close = None;
                // The pointer's x while a tab is dragged, for scrolling at the strip's edges.
                let mut drag_x = None;
                ui.scope(|ui| {
                    ui.style_mut().always_scroll_the_only_direction = true;
                    let mut area = egui::ScrollArea::horizontal().id_salt("document_tabs").max_width(tabs_width).auto_shrink([true, true]);
                    if let Some(x) = ui.data_mut(|d| d.remove_temp::<f32>(scroll_id.with("to"))) {
                        area = area.horizontal_scroll_offset(x);
                    }
                    let out = area.show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            // Each document tab's index and rect, and the one being dragged
                            // (with whether it was just released).
                            let mut slots = Vec::with_capacity(app.views.len());
                            let mut dragged = None;
                            for i in 0..app.views.len() {
                                let Some(doc) = app.session.get(app.views[i].id) else { continue };
                                let Some(name) = crate::canvas::tab_label(app, i) else { continue };
                                let dirty = doc.dirty;
                                // A tab showing the document's title names its file on hover.
                                let file = (name != doc.name).then_some(doc.name.as_str());
                                let response = tab(
                                    ui,
                                    TabSpec {
                                        id: ui.scope_id().with(("document-tab", app.views[i].id)),
                                        theme: &t,
                                        icon: "file-text",
                                        name: &name,
                                        file,
                                        dirty,
                                        active: app.active == Some(i),
                                        index: i,
                                        cap,
                                        height: tab_h,
                                        draggable: true,
                                        grouped: app.split.is_some_and(|s| s.left == app.views[i].id || s.right == app.views[i].id),
                                    },
                                    &mut close,
                                );
                                slots.push((i, response.rect));
                                if response.dragged() || response.drag_stopped() {
                                    dragged = Some((i, response.drag_stopped()));
                                }
                                if changed && app.active == Some(i) && !app.combine_showing() {
                                    response.scroll_to_me(Some(Align::Center));
                                }
                                let id = app.views[i].id;
                                let reveal = cfg!(not(target_arch = "wasm32")) && doc.path.is_some();
                                response.context_menu(|ui| {
                                    if ui.button(tl!("Close Tab")).clicked() {
                                        action = Some(crate::tab_context::Action::Close(id));
                                        ui.close();
                                    }
                                    if ui.add_enabled(!app.closed_tabs.is_empty(), egui::Button::new(tl!("Reopen Closed Tab"))).clicked() {
                                        action = Some(crate::tab_context::Action::Reopen);
                                        ui.close();
                                    }
                                    let label = if cfg!(target_os = "macos") { tl!("Reveal in Finder") } else { tl!("Reveal in File Explorer") };
                                    if ui.add_enabled(reveal, egui::Button::new(label)).clicked() {
                                        action = Some(crate::tab_context::Action::Reveal(id));
                                        ui.close();
                                    }
                                });
                                if response.clicked() {
                                    app.active = Some(i);
                                }
                                // A middle click closes the tab, as in Acrobat and browsers (#1853);
                                // unsaved changes ask first, as the close button does.
                                if response.middle_clicked() {
                                    close = Some(i);
                                }
                            }
                            // Dragging a tab moves it (#1537): a line marks the gap it will land
                            // in, and releasing the button drops it there.
                            if let Some((from, released)) = dragged
                                && let Some(x) = ui.ctx().pointer_latest_pos().map(|p| p.x)
                            {
                                let to = drop_index(&slots, from, x);
                                if released {
                                    if close.is_none() && action.is_none() {
                                        app.move_tab(from, to);
                                    }
                                } else {
                                    drag_x = Some(x);
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    if let Some(marker) = drop_marker(&slots, from, to, ui.spacing().item_spacing.x) {
                                        let span = slots.first().map_or(ui.max_rect().y_range(), |(_, r)| r.y_range());
                                        // In front of the first tab the gap lies outside the scroll area's
                                        // clip rect (#2225): keep the whole line inside it.
                                        let clip = ui.clip_rect();
                                        let marker = marker.clamp(clip.left() + 1.0, (clip.right() - 1.0).max(clip.left() + 1.0));
                                        ui.painter().vline(marker, span.shrink(4.0), Stroke::new(2.0, t.accent));
                                    }
                                }
                            }
                            if let Some(i) = close
                                && let Some(view) = app.views.get(i)
                            {
                                action.get_or_insert(crate::tab_context::Action::Close(view.id));
                            }
                            if let Some(action) = action {
                                app.tab_action(action);
                            }
                            if app.combine_tab.open {
                                // After the document tabs; its index can't clash with theirs.
                                let mut close = None;
                                let response = tab(
                                    ui,
                                    TabSpec {
                                        id: ui.scope_id().with("combine-tab"),
                                        theme: &t,
                                        icon: "files",
                                        name: tl!("Combine files"),
                                        file: None,
                                        dirty: false,
                                        active: app.combine_showing(),
                                        index: usize::MAX,
                                        cap,
                                        height: tab_h,
                                        draggable: false,
                                        grouped: false,
                                    },
                                    &mut close,
                                );
                                if changed && app.combine_showing() {
                                    response.scroll_to_me(Some(Align::Center));
                                }
                                if response.clicked() {
                                    app.open_combine_tab();
                                }
                                if close.is_some() || response.middle_clicked() {
                                    app.close_combine_tab();
                                }
                            }
                        });
                    });
                    let max = (out.content_size.x - out.inner_rect.width()).max(0.0);
                    ui.data_mut(|d| d.insert_temp(scroll_id, (out.state.clamped_offset().x, max)));
                    // A tab dragged to either edge of a strip that scrolls brings the hidden tabs in.
                    if let Some(x) = drag_x
                        && max > 0.0
                    {
                        let step = if x < out.inner_rect.left() + TAB_DRAG_EDGE {
                            -TAB_DRAG_STEP
                        } else if x > out.inner_rect.right() - TAB_DRAG_EDGE {
                            TAB_DRAG_STEP
                        } else {
                            0.0
                        };
                        if step != 0.0 {
                            ui.data_mut(|d| d.insert_temp(scroll_id.with("to"), (out.state.clamped_offset().x + step).clamp(0.0, max)));
                            ui.ctx().request_repaint();
                        }
                    }
                });
                if overflow && arrow(ui, "chevron-right", offset < max_offset - 0.5, tl!("Scroll tabs right")) {
                    scroll_to = Some((offset + TAB_MIN_WIDTH + gap).min(max_offset));
                }
                if let Some(x) = scroll_to {
                    ui.data_mut(|d| d.insert_temp(scroll_id.with("to"), x));
                    ui.ctx().request_repaint();
                }
                ui.add_space(4.0);
                let tip = crate::commands::command_tip(ui.ctx(), tl!("Open a PDF ({key})"), "file.open");
                let open_resp = widgets::ghost_button_sized(ui, "plus", tl!("Open"), tab_h).on_hover_text(tip);
                let mut open_file = None;
                let mut open_as_file = None;
                let mut trigger_browse = false;
                egui::Popup::menu(&open_resp).id(open_resp.id.with("open_dropdown")).show(|ui| {
                    ui.set_min_width(220.0);
                    if ui.add(egui::Button::new(egui::RichText::new(tl!("Browse files…")).font(theme::medium(12.5))).frame(false)).clicked() {
                        trigger_browse = true;
                        ui.close();
                    }
                    if !app.recent.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(tl!("Recent files")).font(theme::semibold(11.0)).color(t.text_muted));
                        for r in app.recent.iter().take(8) {
                            let name = std::path::Path::new(&r.path).file_name().and_then(|n| n.to_str()).unwrap_or(&r.path);
                            let item_btn = ui.add(
                                egui::Button::new(egui::RichText::new(name).font(theme::regular(12.0)))
                                    .frame(false)
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            );
                            egui::Popup::context_menu(&item_btn).id(item_btn.id.with("tab_recent_ctx")).show(|ui| {
                                if ui.button(tl!("Open in Edit Mode")).clicked() {
                                    open_as_file = Some((r.path.clone(), "edit"));
                                    ui.close();
                                }
                                if ui.button(tl!("Organize pages")).clicked() {
                                    open_as_file = Some((r.path.clone(), "organize"));
                                    ui.close();
                                }
                                if ui.button(tl!("Fill & Sign")).clicked() {
                                    open_as_file = Some((r.path.clone(), "fill_sign"));
                                    ui.close();
                                }
                            });
                            if item_btn.on_hover_text(&r.path).clicked() {
                                open_file = Some(r.path.clone());
                                ui.close();
                            }
                        }
                    }
                });
                if trigger_browse {
                    app.open_dialog();
                }
                if let Some((path, tool)) = open_as_file {
                    app.pending_tool_on_open = Some(tool);
                    app.open_path(&path);
                } else if let Some(path) = open_file {
                    app.open_path(&path);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !controls.right.is_empty() {
                        // Laid out left to right inside the right-to-left run, so they read in
                        // the desktop's order.
                        let size = egui::vec2(window_controls::width(&controls.right) - window_controls::SIDE_GAP, window_controls::BUTTON);
                        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            window_controls::buttons(ui, &controls.right);
                        });
                        ui.add_space(window_controls::SIDE_GAP - 4.0);
                    }
                    // The document actions that used to live in the mode bar, now on the
                    // document bar beside the window controls.
                    if icons::button(ui, "settings", button, false, tl!("Preferences")).clicked() {
                        app.execute("app.preferences");
                    }
                    if icons::button(ui, "search", button, false, tl!("Find tools and commands")).clicked() {
                        app.palette_open = true;
                    }
                    let has_doc = app.active.is_some();
                    ui.add_enabled_ui(has_doc, |ui| {
                        if icons::button(ui, "files", button, app.split.is_some(), tl!("Split view")).clicked() {
                            app.run_command("view.split");
                        }
                        if let Some(locked) = app.split.as_ref().map(|s| s.scroll_lock) {
                            let tip = if locked { tl!("Unlock scrolling") } else { tl!("Lock scrolling") };
                            if icons::button(ui, if locked { "lock" } else { "lock-open" }, button, locked, tip).clicked()
                                && let Some(split) = app.split.as_mut()
                            {
                                split.scroll_lock = !locked;
                            }
                        }
                        if icons::button(ui, "info", button, false, tl!("Document properties")).clicked() {
                            app.dialog = Some(Dialog::Properties(PropsTab::Description));
                        }
                        let tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                        if icons::button(ui, "save", button, false, &tip).clicked() {
                            app.run_command("file.save");
                        }
                        let tip = crate::commands::command_tip(ui.ctx(), tl!("Print ({key})"), "print.dialog");
                        // A document that forbids printing says so here, before the dialog (#2669).
                        let may_print = pdfcraft_engine::commands::command("print.dialog").is_some_and(|c| app.command_enabled(c));
                        let mut print = ui.add_enabled_ui(may_print, |ui| icons::button(ui, "printer", button, false, &tip)).inner;
                        if !may_print {
                            print = print.on_disabled_hover_text(if app.active.is_some() {
                                crate::print_refused()
                            } else {
                                tl!("Open a document to print")
                            });
                        }
                        if print.clicked() {
                            app.run_command("print.dialog");
                        }
                    });
                });
            });
        });
}

/// The right-click menu on the title bar's theme button.
fn appearance_menu(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let themes = [
        (ThemePreference::System, tl!("Use system setting")),
        (ThemePreference::Light, tl!("Light gray")),
        (ThemePreference::Dark, tl!("Dark gray")),
        (ThemePreference::Sepia, tl!("Sepia")),
        (ThemePreference::Nordic, tl!("Nordic")),
        (ThemePreference::Dracula, tl!("Dracula")),
    ];
    for (pref, label) in themes {
        let checked = app.theme_preference == pref;
        if ui.selectable_label(checked, label).clicked() {
            app.set_theme_preference(pref);
            ui.close();
        }
    }
    ui.separator();
    let mut mono = app.monochrome;
    if ui.add(crate::widgets::checkbox(&mut mono, tl!("Monochrome"))).changed() {
        app.set_monochrome(mono);
    }
    ui.separator();
    if ui.button(tl!("Appearance settings…")).clicked() {
        app.dialog = Some(Dialog::Preferences);
        ui.close();
    }
}

/// A tab's chrome around its name: the icon on the left, the close button on the right.
const TAB_CHROME: f32 = 64.0;
/// The narrowest a tab shrinks to before the strip scrolls: room for most of a name.
const TAB_MIN_WIDTH: f32 = 180.0;
/// The arrows either side of tabs that don't fit.
const TAB_ARROW: f32 = 24.0;

/// Whether tabs of `natural` widths, `gap` apart, are wider than `budget` even at `min` each.
fn tabs_overflow(natural: &[f32], budget: f32, gap: f32, min: f32) -> bool {
    let narrowest: f32 = natural.iter().map(|w| w.min(min)).sum();
    narrowest + gap * natural.len().saturating_sub(1) as f32 > budget
}

/// How close to either end of a scrolling strip a dragged tab scrolls it, and how far per frame.
const TAB_DRAG_EDGE: f32 = 24.0;
const TAB_DRAG_STEP: f32 = 8.0;

/// Where the tab at `from` lands when dropped with the pointer at `x`: past every other tab
/// whose centre is left of the pointer. `slots` are the tabs' indices and rects, in order.
fn drop_index(slots: &[(usize, Rect)], from: usize, x: f32) -> usize {
    slots.iter().filter(|(i, r)| *i != from && r.center().x < x).count()
}

/// The x of the line marking where the tab at `from` drops at `to`: in the gap before the other
/// tab it lands in front of, or after the last. `None` when it would stay where it is.
fn drop_marker(slots: &[(usize, Rect)], from: usize, to: usize, gap: f32) -> Option<f32> {
    if slots.iter().position(|(i, _)| *i == from) == Some(to) {
        return None;
    }
    let mut others = slots.iter().filter(|(i, _)| *i != from).map(|(_, r)| r);
    match to.checked_sub(1) {
        None => others.next().map(|r| r.left() - gap / 2.0),
        Some(before) => others.nth(before).map(|r| r.right() + gap / 2.0),
    }
}

/// A scroll arrow beside the tabs; dimmed (and inert) at its end. True when clicked.
fn arrow(ui: &mut egui::Ui, icon: &str, enabled: bool, tip: &str) -> bool {
    ui.add_enabled_ui(enabled, |ui| icons::button(ui, icon, TAB_ARROW, false, tip)).inner.clicked()
}

/// A tab name, at most 28 characters (longer names end in "…").
fn tab_name(name: &str) -> String {
    if name.chars().count() > 28 { format!("{}…", name.chars().take(27).collect::<String>()) } else { name.to_string() }
}

/// The width a tab takes when nothing has to shrink.
fn tab_natural_width(ui: &egui::Ui, t: &Tokens, name: &str) -> f32 {
    let label = tab_name(name);
    ui.fonts_mut(|f| f.layout_no_wrap(label, theme::regular(13.0), t.text).size().x) + TAB_CHROME
}

/// The widest any tab may be so that tabs of `natural` widths, `gap` apart, fit in `budget`:
/// narrow tabs keep their width and the rest share what is left equally (as browsers do).
/// `None` when every tab fits as it is; never below `min` (then the strip scrolls).
fn tab_cap(natural: &[f32], budget: f32, gap: f32, min: f32) -> Option<f32> {
    let n = natural.len();
    if n == 0 {
        return None;
    }
    let mut left = budget - gap * (n - 1) as f32;
    if natural.iter().sum::<f32>() <= left {
        return None;
    }
    let mut sorted = natural.to_vec();
    sorted.sort_by(f32::total_cmp);
    for (k, w) in sorted.iter().enumerate() {
        let share = left / (n - k) as f32;
        if *w > share {
            return Some(share.max(min));
        }
        left -= w;
    }
    None
}

/// Draw one tab with its icon, elided label, close button and unsaved dot; records a close request and
/// returns the tab response for drag handling.
struct TabSpec<'a> {
    id: egui::Id,
    theme: &'a Tokens,
    icon: &'a str,
    name: &'a str,
    file: Option<&'a str>,
    dirty: bool,
    active: bool,
    index: usize,
    cap: Option<f32>,
    height: f32,
    draggable: bool,
    /// This tab is one of the two split-view documents; draw the group's accent.
    grouped: bool,
}

fn tab(ui: &mut egui::Ui, spec: TabSpec<'_>, close: &mut Option<usize>) -> egui::Response {
    let TabSpec { id, theme: t, icon, name, file, dirty, active, index, cap, height, draggable, grouped } = spec;
    let font = theme::regular(13.0);
    let mut label = tab_name(name);
    // A shrunk tab shows as much of its name as fits (the tooltip has all of it).
    if let Some(cap) = cap {
        let room = (cap - TAB_CHROME).max(0.0);
        label = crate::panels::ellipsized_prefix(&label, |s| crate::bidi::layout_ltr(ui, s.to_owned(), font.clone(), t.text).size().x <= room);
    }
    // Logical text, laid out explicitly left to right: the fork shapes and reorders it once,
    // and an Arabic file name keeps its extension at the visual end. AccessKit names stay logical.
    let label_color = if active { t.text } else { t.text_muted };
    let label_galley = crate::bidi::layout_ltr(ui, label.clone(), font.clone(), label_color);
    let text_w = label_galley.size().x;
    let width = cap.map_or(text_w + TAB_CHROME, |cap| (text_w + TAB_CHROME).min(cap));
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    if grouped {
        // The group line is drawn for both split tabs, but only the focused one carries the
        // accent; the other gets a faint rule so the bar cannot point at the wrong PDF.
        let color = if active { t.accent } else { t.border };
        ui.painter().hline(rect.x_range().shrink(5.0), rect.bottom() - 1.0, Stroke::new(2.0, color));
    }
    let a11y = if dirty { format!("{name} (edited)") } else { name.to_string() };
    let sense = if draggable { Sense::click_and_drag() } else { Sense::click() };
    let resp = craft_ui::tabs::Tab::new(id, &a11y, active).sense(sense).focus_stroke(Stroke::new(1.0, t.accent)).show_at(ui, rect, |ui, resp| {
        let press = theme::Press::track(ui, resp);
        let bg = if active {
            press.fill(t.chrome, t.chrome.lerp_to_gamma(t.hover, 0.65), t.hover)
        } else {
            press.fill(Color32::TRANSPARENT, t.hover, t.pressed)
        };
        if bg.a() > 0 {
            ui.painter().rect_filled(rect, CornerRadius { nw: 7, ne: 7, sw: 0, se: 0 }, bg);
        }
        icons::paint(
            ui,
            Rect::from_min_size(rect.min + vec2(6.0, (height - 16.0) / 2.0), vec2(16.0, 16.0)),
            icon,
            15.0,
            if active { t.accent } else { t.text_muted },
        );
        ui.painter().galley(rect.min + vec2(28.0, (rect.height() - label_galley.size().y) / 2.0), label_galley.clone(), label_color);
    });
    let x_rect = Rect::from_center_size(rect.right_center() - vec2(16.0, 0.0), vec2(20.0, 20.0));
    let x = ui.interact(x_rect, id.with("close"), Sense::click());
    let x_press = theme::Press::track(ui, &x);
    x_press.wash(ui, x_rect, 4, false);
    // Unsaved changes: a dot where the close button sits, until the tab is hovered.
    if dirty && !resp.hovered() && !x.hovered() {
        ui.painter().circle_filled(x_rect.center(), 4.0, if active { t.text } else { t.text_muted });
    } else if active || resp.hovered() || x.hovered() {
        icons::paint(ui, x_rect.translate(x_press.offset()), "x", 13.0, t.text_muted);
    }
    if theme::hand(x).clicked() {
        *close = Some(index);
    }
    let shown = match file {
        Some(file) => format!("{name}\n{file}"),
        None => name.to_string(),
    };
    theme::hand(resp).on_hover_text(if dirty { crate::i18n::fmt(tl!("{name} — unsaved changes"), &[("name", &shown)]) } else { shown })
}

#[cfg(test)]
mod tab_widths {
    use super::{MAC_CONTENT_LEFT, MAC_LIGHT, MAC_LIGHT_PITCH, MAC_LIGHTS_LEFT, MAC_TITLE_BAR, tab_cap};

    /// Tabs that fit keep their own widths.
    #[test]
    fn tabs_that_fit_do_not_shrink() {
        assert_eq!(tab_cap(&[200.0, 150.0], 400.0, 4.0, 110.0), None);
        assert_eq!(tab_cap(&[], 0.0, 4.0, 110.0), None);
    }

    /// Too wide: narrow tabs keep their width, the wide ones share the rest equally, and
    /// together they fill the strip exactly.
    #[test]
    fn wide_tabs_share_what_narrow_ones_leave() {
        let natural = [120.0, 300.0, 300.0];
        let cap = tab_cap(&natural, 500.0, 4.0, 110.0).expect("must shrink");
        let used: f32 = natural.iter().map(|w| w.min(cap)).sum::<f32>() + 2.0 * 4.0;
        assert!((used - 500.0).abs() < 0.01, "{cap} uses {used}");
        assert!((cap - 186.0).abs() < 0.01, "{cap}");
    }

    /// Arrows appear only when the tabs can't fit even at their narrowest.
    #[test]
    fn arrows_only_when_the_narrowest_tabs_overflow() {
        use super::tabs_overflow;
        assert!(!tabs_overflow(&[250.0; 3], 600.0, 4.0, 180.0), "three shrunk tabs fit");
        assert!(tabs_overflow(&[250.0; 4], 600.0, 4.0, 180.0), "four don't, even at 180");
        assert!(!tabs_overflow(&[100.0; 5], 600.0, 4.0, 180.0), "narrow tabs fit as they are");
        assert!(!tabs_overflow(&[], 0.0, 4.0, 180.0));
    }

    /// Tabs never get narrower than the minimum: past it, the strip scrolls instead.
    #[test]
    fn tabs_stop_at_their_minimum() {
        assert_eq!(tab_cap(&[250.0; 12], 600.0, 4.0, 110.0), Some(110.0));
        assert_eq!(tab_cap(&[250.0; 3], 0.0, 4.0, 110.0), Some(110.0));
    }

    /// Three 100-wide tabs, 4 apart, starting at x = 0.
    fn three_slots() -> Vec<(usize, egui::Rect)> {
        (0..3).map(|i| (i, egui::Rect::from_min_size(egui::pos2(i as f32 * 104.0, 0.0), egui::vec2(100.0, 30.0)))).collect()
    }

    /// A dragged tab lands past every other tab whose centre the pointer has crossed.
    #[test]
    fn a_dragged_tab_drops_past_the_centres_it_crossed() {
        use super::drop_index;
        let slots = three_slots();
        // Centres at 50, 154 and 258.
        assert_eq!(drop_index(&slots, 0, 10.0), 0, "still over itself");
        assert_eq!(drop_index(&slots, 0, 160.0), 1, "past the middle tab's centre");
        assert_eq!(drop_index(&slots, 0, 900.0), 2, "past the end");
        assert_eq!(drop_index(&slots, 2, -50.0), 0, "before the start");
        assert_eq!(drop_index(&slots, 2, 100.0), 1, "between the first and middle centres");
        assert_eq!(drop_index(&[], 0, 100.0), 0);
    }

    /// The drop line sits in the gap the tab will land in, and not at all where it already is.
    #[test]
    fn the_drop_line_marks_the_gap_and_hides_when_nothing_moves() {
        use super::drop_marker;
        let slots = three_slots();
        assert_eq!(drop_marker(&slots, 0, 0, 4.0), None);
        assert_eq!(drop_marker(&slots, 1, 1, 4.0), None);
        // The first tab dropped at the end: after the last tab.
        assert_eq!(drop_marker(&slots, 0, 2, 4.0), Some(310.0));
        // The first tab dropped between the middle and the last: in the gap between them.
        assert_eq!(drop_marker(&slots, 0, 1, 4.0), Some(206.0));
        // The last tab dropped at the start: before the first tab.
        assert_eq!(drop_marker(&slots, 2, 0, 4.0), Some(-2.0));
        assert_eq!(drop_marker(&[], 0, 1, 4.0), None);
    }

    #[test]
    fn the_mac_title_bar_centres_and_clears_the_traffic_lights() {
        // The lights are centred in the bar, and content keeps the same gap after them (#789).
        let margin = (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;
        let lights_end = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT;
        assert_eq!(MAC_CONTENT_LEFT - lights_end, margin);
        // `egui::Margin` is in `i8`.
        assert!(MAC_CONTENT_LEFT <= f32::from(i8::MAX));
    }
}
