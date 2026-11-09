//! The tools drawer: one overlay list behind the hamburger, where every catalog group expands in
//! place instead of replacing the list. It also holds the workspace switch and the editor for the
//! reader's own annotation row.

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, pos2, vec2};
use pdfcraft_engine::catalog::{self, Availability, TOOL_CATEGORIES, TOOL_GROUPS, ToolGroup};

use crate::theme::{self, Tokens};
use crate::{Mode, PdfCraftApp, icons};

/// Groups whose tools need their options pane open beside the page.
const PANEL_GROUPS: &[&str] = &["organize", "form", "stamp", "measure", "redact", "edit"];

/// Draw the drawer over the document area. `main` is the band the page occupies, below the
/// window's two rows.
pub(crate) fn show(app: &mut PdfCraftApp, ctx: &egui::Context, main: Rect) {
    if !app.drawer_open {
        return;
    }
    if !app.modal_open() && !app.palette_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.drawer_open = false;
        return;
    }
    let t = Tokens::get(ctx);
    let tab_strip_bottom = if cfg!(target_os = "macos") && app.integrated_titlebar { crate::chrome::MAC_TITLE_BAR } else { 38.0 };
    let dock = app.hotbar.dock.as_deref().unwrap_or("top");
    let (left, top, height, width) = match dock {
        "left" => {
            let left = app.strip_rect.map_or(main.left(), |r| r.right());
            let top = tab_strip_bottom;
            let height = (main.bottom() - top).max(0.0);
            let width = 344.0f32.min((main.right() - left).max(200.0));
            (left, top, height, width)
        }
        "right" => {
            let top = tab_strip_bottom;
            let height = (main.bottom() - top).max(0.0);
            let right = app.strip_rect.map_or(main.right(), |r| r.left());
            let width = 344.0f32.min((right - main.left()).max(200.0));
            let left = (right - width).max(main.left());
            (left, top, height, width)
        }
        "bottom" => {
            let left = main.left();
            let top = tab_strip_bottom;
            let bottom = app.strip_rect.map_or(main.bottom(), |r| r.top());
            let height = (bottom - top).max(0.0);
            let width = 344.0f32.min((main.right() - left).max(200.0));
            (left, top, height, width)
        }
        _ => {
            let top = app.strip_rect.map_or(tab_strip_bottom, |r| r.bottom().max(tab_strip_bottom));
            let left = main.left();
            let height = (main.bottom() - top).max(0.0);
            let width = 344.0f32.min((main.right() - left).max(200.0));
            (left, top, height, width)
        }
    };

    // A click on the page while the drawer is open closes it; it also keeps page input from
    // reaching the document underneath.
    // The scrim covers everything under the row, side panels included, so the drawer is the only
    // thing in focus and a click anywhere else closes it.
    let screen = ctx.content_rect();
    let scrim_rect = Rect::from_min_max(pos2(screen.left(), top), pos2(screen.right(), top + height));
    let scrim = egui::Area::new(egui::Id::unique("drawer-scrim")).order(egui::Order::Middle).fixed_pos(scrim_rect.min).show(ctx, |ui| {
        let (rect, resp) = ui.allocate_exact_size(scrim_rect.size(), Sense::click());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(if t.dark() { 90 } else { 45 }));
        resp
    });
    if scrim.inner.clicked() {
        app.drawer_open = false;
        return;
    }

    egui::Area::new(egui::Id::unique("drawer")).order(egui::Order::Foreground).fixed_pos(pos2(left, top)).constrain(false).show(ctx, |ui| {
        ui.set_width(width);
        ui.set_height(height);
        let frame = egui::Frame::NONE
            .fill(t.panel)
            .stroke(Stroke::new(1.0, t.border))
            .shadow(egui::Shadow { offset: [6, 0], blur: 24, spread: 0, color: Color32::from_black_alpha(if t.dark() { 140 } else { 40 }) })
            .inner_margin(egui::Margin { left: 14, right: 14, top: 12, bottom: 12 });
        frame.show(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.set_min_height(height - 24.0);
            header(app, ui, &t);
            ui.add_space(8.0);
            search(app, ui, &t);
            ui.add_space(8.0);
            if app.customizing_toolbar {
                toolbar_editor(app, ui, &t);
            } else {
                workspace(app, ui, &t);
                ui.add_space(6.0);
                catalog(app, ui, &t);
            }
        });
    });
}

fn header(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Tools")).font(theme::semibold(17.0)).color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, "x", 26.0, false, tl!("Close")).clicked() {
                app.drawer_open = false;
            }
            let tip = if app.customizing_toolbar { tl!("Done editing the toolbar") } else { tl!("Edit the annotation row") };
            if icons::button(ui, "pencil", 26.0, app.customizing_toolbar, tip).clicked() {
                app.customizing_toolbar = !app.customizing_toolbar;
            }
        });
    });
}

fn search(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let field = egui::Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(8, 5));
    field.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add(icons::image("search", 14.0, t.text_faint));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.all_tools_query)
                    .hint_text(tl!("Search tools and commands"))
                    .frame(egui::Frame::NONE)
                    .desired_width(ui.available_width()),
            );
            if resp.has_focus() && !app.all_tools_query.is_empty() {
                // The result list follows each keystroke.
                ui.ctx().request_repaint();
            }
        });
    });
}

/// The workspace switch, folded away: reading does not need it until a conversion or a signing
/// session does.
fn workspace(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let current = match app.mode {
        Mode::AllTools => tl!("All tools"),
        Mode::Read => tl!("Read"),
        Mode::Edit => tl!("Edit"),
        Mode::Convert => tl!("Convert"),
        Mode::Sign => tl!("E-Sign"),
    };
    egui::CollapsingHeader::new(egui::RichText::new(crate::i18n::fmt(tl!("Workspace · {mode}"), &[("mode", current)])).color(t.text))
        .id_salt("drawer-workspace")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (mode, label) in
                    [(Mode::Read, "Read"), (Mode::Edit, "Edit"), (Mode::Convert, "Convert"), (Mode::Sign, "E-Sign"), (Mode::AllTools, "All tools")]
                {
                    let chip = ui.selectable_label(app.mode == mode, tl!(label));
                    chip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Workspace {label}")));
                    if chip.clicked() {
                        if mode == Mode::AllTools {
                            // All tools is the drawer itself; keep it open, no second panel.
                            app.switch_mode(mode, false);
                            app.left = crate::LeftPanel::AllTools;
                            app.left_open = false;
                            app.all_tools_full = false;
                        } else {
                            // The workspace's own panel opens; the All-tools drawer closes so it
                            // cannot be left floating over that panel.
                            app.switch_mode(mode, true);
                            app.drawer_open = false;
                        }
                    }
                }
            });
        });
    ui.add_space(4.0);
}

/// The catalog as boxed tiles: every group is a card, and an open group drops its item tiles
/// underneath its row, pushing the rest of the catalog down.
fn catalog(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let query = app.all_tools_query.trim().to_lowercase();
    let mut chosen: Option<&'static str> = None;
    let list_height = (ui.available_height() - 104.0).max(120.0);
    egui::ScrollArea::vertical().id_salt("drawer-catalog").max_height(list_height).auto_shrink([false, false]).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let pinned: Vec<&ToolGroup> = app.pinned_tools.iter().filter_map(|id| catalog::group(id)).collect();
        if !pinned.is_empty() && query.is_empty() {
            ui.label(egui::RichText::new(tl!("Pinned")).font(theme::semibold(11.5)).color(t.text_faint));
            ui.add_space(4.0);
            group_grid(app, ui, t, &pinned, &query, &mut chosen);
            ui.add_space(6.0);
        }
        for category in TOOL_CATEGORIES {
            let groups: Vec<&ToolGroup> = category.groups.iter().filter_map(|id| catalog::group(id)).collect();
            let visible: Vec<&ToolGroup> = groups
                .iter()
                .copied()
                .filter(|g| group_matches(g, &query))
                .filter(|g| !(query.is_empty() && pinned.iter().any(|p| p.id == g.id)))
                .collect();
            if visible.is_empty() {
                continue;
            }
            ui.add_space(6.0);
            ui.label(egui::RichText::new(tl!(category.label)).font(theme::semibold(11.5)).color(t.text_faint));
            ui.add_space(3.0);
            group_grid(app, ui, t, &visible, &query, &mut chosen);
        }
        if !TOOL_GROUPS.iter().any(|g| group_matches(g, &query)) {
            ui.add_space(16.0);
            ui.label(egui::RichText::new(tl!("No tools match your search.")).color(t.text_muted));
        }
        ui.add_space(20.0);
    });
    links(app, ui, t);
    if let Some(id) = chosen {
        app.select_tool(id);
        app.drawer_open = false;
    }
}

/// Group tiles, two to a row. An open group's items are drawn under that row, full width.
fn group_grid(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, groups: &[&'static ToolGroup], query: &str, chosen: &mut Option<&'static str>) {
    let gap = 8.0;
    let width = ((ui.available_width() - gap) / 2.0).max(80.0);
    for pair in groups.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for group in pair {
                group_tile(app, ui, t, group, width, query);
            }
        });
        for group in pair {
            if app.drawer_expanded.as_deref() == Some(group.id) || !query.is_empty() {
                item_grid(app, ui, t, group, query, chosen);
            }
        }
        ui.add_space(2.0);
    }
}

/// One group card: icon over the name, pinned and open states on the card itself.
fn group_tile(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, group: &'static ToolGroup, width: f32, query: &str) {
    let open = app.drawer_expanded.as_deref() == Some(group.id) || !query.is_empty();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 76.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!(group.label)));
    let press = theme::Press::track(ui, &resp);
    let fill = if open { t.accent_soft } else { press.fill(t.card, t.hover, t.pressed) };
    let stroke = if open { Stroke::new(1.0, t.accent) } else { Stroke::new(1.0, t.divider) };
    ui.painter().rect(rect, CornerRadius::same(9), fill, stroke, egui::StrokeKind::Inside);
    let body = rect.translate(press.offset());
    let ready = group.availability == Availability::Ready;
    icons::paint(
        ui,
        Rect::from_center_size(body.center_top() + vec2(0.0, 23.0), vec2(26.0, 26.0)),
        group.icon,
        24.0,
        if ready { t.tool_tint(group.hue) } else { t.text_faint },
    );
    let mut job = egui::text::LayoutJob::simple(
        tl!(group.label).to_owned(),
        theme::regular(11.5),
        if ready { t.text } else { t.text_muted },
        (body.width() - 12.0).max(1.0),
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let elided = galley.elided;
    ui.painter().galley(egui::pos2(body.center().x, body.top() + 41.0), galley, t.text);

    // Pinned state, and the pin on hover so the card stays calm.
    let pinned = app.is_tool_pinned(group.id);
    let pin_rect = Rect::from_center_size(body.right_top() + vec2(-13.0, 13.0), vec2(20.0, 20.0));
    // A real widget, so assistive tech and the control channel can find and activate the pin even
    // though it is painted only on hover or when pinned.
    let tip = if pinned { tl!("Unpin from Home") } else { tl!("Pin to Home") };
    let pin_resp = ui.put(pin_rect, egui::Button::new("").frame(false).fill(egui::Color32::TRANSPARENT)).on_hover_text(tip);
    pin_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
    let mut pin_clicked = false;
    if pin_resp.hovered() {
        ui.painter().rect_filled(pin_rect, CornerRadius::same(4), t.pressed);
    }
    if pinned || resp.hovered() || pin_resp.hovered() {
        icons::paint(ui, pin_rect.shrink(3.0), "pin", 12.0, if pinned { t.accent_text } else { t.text_faint });
    }
    if theme::hand(pin_resp).clicked() {
        app.toggle_pin_tool(group.id);
        pin_clicked = true;
    }
    if let Some(_badge) = group.badge.filter(|_| ready) {
        ui.painter().circle_filled(body.right_top() + vec2(-9.0, 9.0), 3.0, t.badge_new);
    }
    let tip = match group.availability {
        Availability::Ready => tl!("Available").to_string(),
        Availability::Planned(m) => format!("{} {m}", tl!("Planned for milestone")),
        Availability::Provider => tl!("Needs an AI provider you configure").to_owned(),
    };
    let tip = if elided { format!("{} — {tip}", tl!(group.label)) } else { tip };
    let resp = theme::hand(resp).on_hover_text(tip);
    let pin_label = if pinned { tl!("Unpin from Home") } else { tl!("Pin to Home") };
    resp.context_menu(|ui| {
        if ui.button(pin_label).clicked() {
            app.toggle_pin_tool(group.id);
            ui.close();
        }
    });
    if !pin_clicked && resp.clicked() {
        app.drawer_expanded = if open { None } else { Some(group.id.to_owned()) };
    }
}

/// A group's items as small boxed tiles under its row; a disabled tool stays visible but muted.
fn item_grid(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, group: &'static ToolGroup, query: &str, chosen: &mut Option<&'static str>) {
    let items: Vec<&'static pdfcraft_engine::catalog::ToolItem> =
        group.sections.iter().flat_map(|s| s.items.iter()).filter(|i| item_matches(i.label, i.command, query)).collect();
    if items.is_empty() {
        return;
    }
    let outer = ui.available_width();
    let inner = (outer - 16.0).max(1.0);
    let gap = 6.0;
    let width = ((inner - 2.0 * gap) / 3.0).max(56.0);
    egui::Frame::NONE.fill(t.field).stroke(Stroke::new(1.0, t.border)).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::same(8)).show(
        ui,
        |ui| {
            ui.set_width(inner);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(gap, gap);
                for item in items {
                    if item_tile(app, ui, t, group, item, width, chosen) {
                        // The command ran; the drawer closes from the caller.
                    }
                }
            });
        },
    );
    ui.add_space(4.0);
}

/// One item tile. Returns whether it was clicked.
fn item_tile(
    app: &mut PdfCraftApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    group: &'static ToolGroup,
    item: &'static pdfcraft_engine::catalog::ToolItem,
    width: f32,
    chosen: &mut Option<&'static str>,
) -> bool {
    let ready = item.availability == Availability::Ready;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ready, tl!(item.label)));
    let press = theme::Press::track(ui, &resp);
    let fill = press.fill(t.card, t.hover, t.pressed);
    ui.painter().rect(rect, CornerRadius::same(7), fill, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
    let body = rect.translate(press.offset());
    icons::paint(
        ui,
        Rect::from_center_size(body.center_top() + vec2(0.0, 18.0), vec2(18.0, 18.0)),
        item.icon,
        17.0,
        if ready { t.accent_text } else { t.text_faint },
    );
    let mut job = egui::text::LayoutJob::simple(
        tl!(item.label).to_owned(),
        theme::regular(10.0),
        if ready { t.text } else { t.text_muted },
        (body.width() - 8.0).max(1.0),
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let elided = galley.elided;
    ui.painter().galley(egui::pos2(body.center().x, body.top() + 31.0), galley, t.text);
    let tip = if elided { format!("{} — {}", tl!(item.label), item.command) } else { item.command.to_owned() };
    let resp = theme::hand(resp).on_hover_text(tip);
    if ready && resp.clicked() {
        app.run_command(item.command);
        app.drawer_open = false;
        if PANEL_GROUPS.contains(&group.id) {
            chosen.get_or_insert(group.id);
        }
        true
    } else {
        false
    }
}

/// The community and application links that used to sit in the title bar, under the catalog.
fn links(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.separator();
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tl!("Links & help")).font(theme::semibold(11.5)).color(t.text_faint));
    ui.add_space(2.0);
    for (label, command) in [
        ("Telegram — @TINZUR", "help.telegram"),
        ("Keyboard shortcuts", "help.shortcuts"),
        ("Preferences…", "app.preferences"),
        ("About", "help.about"),
        ("PdfCraft source", "help.github"),
    ] {
        if ui.add(egui::Button::new(egui::RichText::new(tl!(label)).size(12.0)).frame(false)).clicked() {
            app.run_command(command);
            app.drawer_open = false;
        }
    }
    ui.add_space(4.0);
}

/// Edit the reader's annotation row: the comment tools as checkboxes, in catalog order.
fn toolbar_editor(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new(tl!("Annotation row")).font(theme::semibold(14.0)).color(t.text));
    ui.label(egui::RichText::new(tl!("Choose the tools that stay in the top row. Select and Hand are always there.")).color(t.text_muted).small());
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Rows")).font(theme::medium(12.0)).color(t.text));
        if ui.selectable_label(app.strip_rows_auto, tl!("Auto (max 2)")).clicked() {
            app.strip_rows_auto = true;
        }
        for rows in [1u8, 2] {
            let label = if rows == 1 { tl!("1 row") } else { tl!("2 rows") };
            if ui.selectable_label(!app.strip_rows_auto && app.strip_rows == rows, label).clicked() {
                app.strip_rows_auto = false;
                app.strip_rows = rows;
            }
        }
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(crate::i18n::fmt(tl!("Fitting: {n} row(s)"), &[("n", &app.strip_rows_shown.to_string())]))
                .font(theme::regular(11.0))
                .color(t.text_muted),
        );
        ui.add_space(12.0);
        let size_label = ui.label(egui::RichText::new(tl!("Icon size")).font(theme::medium(12.0)).color(t.text));
        let mut size = app.hotbar.button_size;
        if ui.add(egui::Slider::new(&mut size, 22.0..=40.0).show_value(false)).labelled_by(size_label.id).changed() {
            app.hotbar.button_size = size;
        }
        ui.label(egui::RichText::new(format!("{size:.0}")).font(theme::regular(11.0)).color(t.text_muted));
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Position")).font(theme::medium(12.0)).color(t.text));
        for (dock, label) in [(None, "Top"), (Some("bottom"), "Bottom"), (Some("left"), "Left"), (Some("right"), "Right")] {
            let on = app.hotbar.dock.as_deref() == dock;
            if ui.selectable_label(on, tl!(label)).clicked() {
                app.hotbar.dock = dock.map(str::to_owned);
                if let Some(side) = dock {
                    let _ = app.hotbar.place(side);
                }
            }
        }
    });
    ui.label(
        egui::RichText::new(tl!("Auto fills the first row and wraps the rest to a second, at most two; the controls stay in the first row."))
            .color(t.text_muted)
            .small(),
    );
    ui.add_space(6.0);
    let query = app.all_tools_query.trim().to_lowercase();
    egui::ScrollArea::vertical().id_salt("drawer-toolbar-editor").auto_shrink([false, false]).show(ui, |ui| {
        let mut rotate_on = app.strip_tools.iter().any(|t| t == "view.rotate");
        let matches_rotate = query.is_empty() || "rotate view".contains(&query) || tl!("Rotate view").to_lowercase().contains(&query);
        if matches_rotate && ui.add(crate::widgets::checkbox(&mut rotate_on, tl!("Rotate view"))).changed() {
            if rotate_on {
                if !app.strip_tools.iter().any(|t| t == "view.rotate") {
                    app.strip_tools.push("view.rotate".to_owned());
                }
            } else {
                app.strip_tools.retain(|t| t != "view.rotate");
            }
        }
        let mut tools: Vec<_> = crate::comments::ALL.to_vec();
        if !query.is_empty() {
            tools.retain(|tool| {
                let s = tool.label().to_lowercase();
                let i18n = tl!(tool.label()).to_lowercase();
                let cmd = tool.command().to_lowercase();
                s.contains(&query) || i18n.contains(&query) || cmd.contains(&query)
            });
            tools.sort_by_key(|tool| {
                let s = tool.label().to_lowercase();
                let i18n = tl!(tool.label()).to_lowercase();
                if s == query || i18n == query {
                    0
                } else if s.starts_with(&query) || i18n.starts_with(&query) {
                    1
                } else {
                    2
                }
            });
        }
        let any_match = !tools.is_empty();
        for tool in tools {
            let id = tool.command();
            let mut on = app.strip_tools.iter().any(|t| t == id);
            if ui.add(crate::widgets::checkbox(&mut on, tl!(tool.label()))).changed() {
                if on {
                    if !app.strip_tools.iter().any(|t| t == id) {
                        app.strip_tools.push(id.to_owned());
                    }
                } else {
                    app.strip_tools.retain(|t| t != id);
                }
            }
        }
        if !query.is_empty() && !any_match {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(tl!("No tools match your search.")).color(t.text_muted));
        }
        ui.add_space(10.0);
        if ui.button(tl!("Reset to the default row")).clicked() {
            app.strip_tools = crate::default_strip_tools();
        }
    });
}

fn group_matches(group: &ToolGroup, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    label_matches(group.label, query) || group.sections.iter().flat_map(|s| s.items.iter()).any(|i| item_matches(i.label, i.command, query))
}

fn item_matches(label: &str, command: &str, query: &str) -> bool {
    query.is_empty() || label_matches(label, query) || label_matches(command, query)
}

fn label_matches(label: &str, query: &str) -> bool {
    label.to_lowercase().contains(query) || tl!(label).to_lowercase().contains(query)
}
