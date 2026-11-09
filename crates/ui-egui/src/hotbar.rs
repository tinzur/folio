//! A movable, persistent set of shortcuts to the shared command registry and core document tools.
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, vec2};
use pdfcraft_engine::Edit;
use pdfcraft_engine::commands::{COMMANDS, command};
use serde::{Deserialize, Serialize};

use crate::{PdfCraftApp, QuickTool, comments, icons, theme, theme::Tokens};

/// Persistent state and UI state for the customizable document hotbar.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Hotbar {
    pub tools: Vec<String>,
    /// Position as a fraction of the available travel, so resizing keeps the bar visible.
    pub position: [f32; 2],
    pub vertical: bool,
    pub button_size: f32,
    pub dock: Option<String>,
    pub full_side: bool,
    #[serde(default)]
    pub placed: bool,
    #[serde(skip)]
    search: String,
    #[serde(skip)]
    picker_open: bool,
}

impl Default for Hotbar {
    /// Create a default vertical floating hotbar positioned in the document margin.
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            position: [0.0, 0.0],
            vertical: true,
            button_size: 32.0,
            dock: None,
            full_side: false,
            placed: false,
            search: String::new(),
            picker_open: false,
        }
    }
}

impl Hotbar {
    /// Clamp button size and normalized position coordinates, drop unknown docked sides,
    /// and deduplicate/filter tool command identifiers against the command registry.
    pub fn normalize(&mut self) {
        self.button_size = if self.button_size.is_finite() { self.button_size.clamp(24.0, 64.0) } else { 32.0 };
        if self.dock.as_deref().is_some_and(|side| !matches!(side, "left" | "right" | "top" | "bottom")) {
            self.dock = None;
        }
        let mut seen = std::collections::HashSet::new();
        self.tools.retain(|id| command(id).is_some() && seen.insert(id.clone()));
        for coordinate in &mut self.position {
            *coordinate = if coordinate.is_finite() { coordinate.clamp(0.0, 1.0) } else { 0.5 };
        }
        if self.dock.is_some() || self.position != [0.0, 0.0] {
            self.placed = true;
        }
    }

    /// Adopt preferences saved before the hotbar absorbed the quick-action bar (#5890), which have
    /// no `placed` key. A bar left at that release's default (floating, horizontal, top centre)
    /// moves to the new default in the left margin; any other spot is one the user chose, so it
    /// is kept, including the top-left corner.
    pub fn adopt_legacy(&mut self) {
        if self.dock.is_none() && !self.vertical && self.position == [0.5, 0.0] {
            self.position = [0.0, 0.0];
            self.vertical = true;
            self.full_side = false;
            self.placed = false;
        } else {
            self.placed = true;
        }
    }

    /// Parse and validate a comma-separated list of command identifiers for the hotbar.
    pub fn set_tools(&mut self, value: &str) -> Result<(), String> {
        let tools: Vec<String> = value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        if let Some(id) = tools.iter().find(|id| command(id).is_none()) {
            return Err(format!("Unknown hotbar command: {id}"));
        }
        self.tools = tools;
        self.normalize();
        Ok(())
    }

    /// Dock the hotbar to `"left"`, `"right"`, `"top"`, or `"bottom"`, or undock with `"float"`.
    pub fn place(&mut self, side: &str) -> Result<(), String> {
        if side == "float" {
            self.dock = None;
            self.placed = true;
            return Ok(());
        }
        let (position, vertical) = match side {
            "left" => ([0.0, 0.5], true),
            "right" => ([1.0, 0.5], true),
            "top" => ([0.5, 0.0], false),
            "bottom" => ([0.5, 1.0], false),
            _ => return Err("hotbar-side must be float, left, right, top or bottom".into()),
        };
        self.dock = Some(side.to_owned());
        self.position = position;
        self.vertical = vertical;
        self.placed = true;
        Ok(())
    }
}

/// Render the hotbar settings gear button and open the tool picker when clicked.
fn settings_button(ui: &mut egui::Ui, cell: f32, picker_open: &mut bool) -> bool {
    let settings = ui
        .push_id("hotbar-settings", |ui| {
            icons::button(ui, "settings", cell, false, tl!("Hotbar settings")).on_hover_text(tl!("Hotbar settings / add tools"))
        })
        .inner;
    if settings.clicked() {
        *picker_open = true;
        true
    } else {
        false
    }
}

/// Determine where flyout popup menus should open from a toolbar button based on docking and layout.
fn popup_align_for(dock: Option<&str>, vertical: bool) -> egui::RectAlign {
    match dock {
        Some("right") => egui::RectAlign::LEFT_START,
        Some("bottom") => egui::RectAlign::TOP_START,
        Some("top") => egui::RectAlign::BOTTOM_START,
        _ => {
            if vertical {
                egui::RectAlign::RIGHT_START
            } else {
                egui::RectAlign::BOTTOM_START
            }
        }
    }
}

/// Whether `spec` matches the hotbar picker search query across the translated label, canonical
/// English label, or command identifier (case-insensitively).
fn matches_search(spec: &pdfcraft_engine::commands::CommandSpec, query: &str) -> bool {
    let search = query.to_lowercase();
    tl!(spec.label).to_lowercase().contains(&search) || spec.label.to_lowercase().contains(&search) || spec.id.to_lowercase().contains(&search)
}

/// Render the configured custom tool shortcut buttons and their removal context menus.
fn tool_buttons(ui: &mut egui::Ui, app: &PdfCraftApp, tools: &[String], cell: f32, run: &mut Option<&'static str>, remove: &mut Option<usize>) {
    for (index, id) in tools.iter().enumerate() {
        let Some(spec) = command(id) else { continue };
        let active = crate::comments::CommentTool::from_command(id).is_some_and(|tool| app.quick_tool == crate::QuickTool::Comment(tool));
        let button = ui.add_enabled_ui(app.command_enabled(spec), |ui| icons::button(ui, spec.icon, cell, active, tl!(spec.label))).inner;
        if button.clicked() {
            *run = Some(spec.id);
        }
        button.context_menu(|ui| {
            if ui.button(tl!("Remove from hotbar")).clicked() {
                *remove = Some(index);
                ui.close();
            }
        });
    }
}

/// Edit text and Fill & Sign, the tail of an annotation row.
fn tool_row_tail(ui: &mut egui::Ui, app: &mut PdfCraftApp, cell: f32, t: &Tokens, popup_align: egui::RectAlign) {
    ui.separator();
    // Edit text & images, while the Edit workspace is open (Acrobat's Edit toolbar)
    // or the tool is in use.
    if app.mode == crate::Mode::Edit || app.quick_tool == QuickTool::EditText {
        render_edit_text_button(app, ui, cell);
    }
    render_fill_sign_button(app, ui, cell, t, popup_align);
}

fn render_edit_text_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let on = app.quick_tool == QuickTool::EditText;
    if icons::button(ui, "text-cursor-input", cell, on, tl!("Edit text & images")).clicked() {
        if on {
            app.put_down_edit_text();
        } else {
            app.quick_tool = QuickTool::EditText;
        }
    }
}

fn render_fill_sign_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, t: &Tokens, popup_align: egui::RectAlign) {
    let current_fill = match app.quick_tool {
        QuickTool::Fill(f) => Some(f),
        _ => None,
    };
    let shown = current_fill.unwrap_or(crate::fill_sign::FillTool::Text);
    let resp = icons::button(ui, if current_fill.is_some() { shown.icon() } else { "pen-line" }, cell, current_fill.is_some(), tl!("Fill & Sign"));
    let r = resp.rect;
    let tri = [r.right_bottom() + vec2(-4.0, -4.0), r.right_bottom() + vec2(-9.0, -4.0), r.right_bottom() + vec2(-4.0, -9.0)];
    ui.painter().add(egui::Shape::convex_polygon(tri.to_vec(), if current_fill.is_some() { t.accent } else { t.text_faint }, Stroke::NONE));
    if resp.clicked() && current_fill.is_none() {
        app.execute(shown.command());
    }
    let open = (resp.clicked() && current_fill.is_some()) || resp.secondary_clicked();
    egui::Popup::menu(&resp).open_memory(open.then_some(egui::SetOpenCommand::Toggle)).align(popup_align).gap(6.0).show(|ui| {
        for tool in crate::fill_sign::FILL_TOOLS {
            if matches!(tool, crate::fill_sign::FillTool::Signature | crate::fill_sign::FillTool::Initials) {
                continue;
            }
            let on = current_fill == Some(tool);
            let (row, click) = ui.allocate_exact_size(vec2(180.0, 28.0), Sense::click());
            let press = theme::Press::track(ui, &click);
            press.wash(ui, row, 4, on);
            let body = row.translate(press.offset());
            icons::paint(ui, Rect::from_min_size(body.min + vec2(8.0, 6.0), vec2(16.0, 16.0)), tool.icon(), 16.0, t.text);
            let reserve = if on { 28.0 } else { 8.0 };
            crate::widgets::paint_left(
                ui,
                body.left_center() + vec2(34.0, 0.0),
                tl!(tool.label()),
                theme::regular(13.0),
                t.text,
                (row.width() - 34.0 - reserve).max(0.0),
            );
            if on {
                icons::paint(ui, Rect::from_min_size(body.right_top() + vec2(-24.0, 7.0), vec2(14.0, 14.0)), "check", 14.0, t.accent);
            }
            let click = theme::hand(click);
            let info = tl!(tool.label()).to_string();
            click.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, info.clone()));
            if click.clicked() {
                app.execute(tool.command());
                ui.close();
            }
        }
        ui.separator();
        if let Some(command) = crate::fill_sign::signature_entries(ui, app, t) {
            app.execute(command);
            ui.close();
        }
    });
}

/// Render the quick tools: Select and Hand, then the annotation tools — every one its own button
/// in the top strip (`direct`), or Acrobat's three grouped flyouts in the compact floating
/// palette — and finally Edit text and Fill & Sign.
fn core_tool_buttons(ui: &mut egui::Ui, app: &mut PdfCraftApp, cell: f32, popup_align: egui::RectAlign, t: &Tokens, direct: bool) {
    if icons::button(ui, "mouse-pointer-2", cell, app.quick_tool == QuickTool::Select, tl!("Select (V)")).clicked() {
        app.quick_tool = QuickTool::Select;
    }
    if icons::button(ui, "hand", cell, app.quick_tool == QuickTool::Hand, tl!("Hand (H)")).clicked() {
        app.quick_tool = QuickTool::Hand;
    }
    if direct {
        // The reader's own short list, edited in the tools drawer. Select and Hand stay fixed.
        let tools = app.strip_tools.clone();
        ui.separator();
        for id in &tools {
            let Some(spec) = pdfcraft_engine::commands::command(id) else { continue };
            if id == "comment.ink" || id == "comment.square" {
                ui.separator();
            }
            let active = comments::CommentTool::from_command(id).is_some_and(|tool| app.quick_tool == QuickTool::Comment(tool));
            if icons::button(ui, spec.icon, cell, active, tl!(spec.label)).clicked() {
                app.execute(id);
            }
        }
        ui.separator();
        if icons::button(ui, "sliders-horizontal", cell, false, tl!("Customize toolbar")).clicked() {
            app.drawer_open = true;
            app.customizing_toolbar = true;
        }
    } else {
        // The floating palette stays compact: Comment ▸, Highlight ▸, Draw ▸. Clicking a group
        // selects its last-used tool; clicking it again opens the flyout.
        for g in 0..comments::GROUPS.len() {
            let current = app.comment_prefs.group_tool[g];
            let active = matches!(app.quick_tool, QuickTool::Comment(t) if t.group() == g);
            let resp = icons::button(ui, current.icon(), cell, active, tl!(current.label()));
            if comments::GROUPS[g].len() == 1 {
                if resp.clicked() {
                    app.execute(current.command());
                }
                continue;
            }
            // A small corner triangle marks the flyout.
            let r = resp.rect;
            let tri = [r.right_bottom() + vec2(-4.0, -4.0), r.right_bottom() + vec2(-9.0, -4.0), r.right_bottom() + vec2(-4.0, -9.0)];
            ui.painter().add(egui::Shape::convex_polygon(tri.to_vec(), if active { t.accent } else { t.text_faint }, Stroke::NONE));
            let open = (resp.clicked() && active) || resp.secondary_clicked();
            if resp.clicked() && !active {
                app.execute(current.command());
            }
            egui::Popup::menu(&resp).open_memory(open.then_some(egui::SetOpenCommand::Toggle)).align(popup_align).gap(6.0).show(|ui| {
                for tool in comments::GROUPS[g] {
                    let on = app.quick_tool == QuickTool::Comment(*tool);
                    let (row, click) = ui.allocate_exact_size(vec2(180.0, 28.0), Sense::click());
                    let press = theme::Press::track(ui, &click);
                    press.wash(ui, row, 4, on);
                    let body = row.translate(press.offset());
                    icons::paint(ui, Rect::from_min_size(body.min + vec2(8.0, 6.0), vec2(16.0, 16.0)), tool.icon(), 16.0, t.text);
                    let reserve = if on { 28.0 } else { 8.0 };
                    crate::widgets::paint_left(
                        ui,
                        body.left_center() + vec2(34.0, 0.0),
                        tl!(tool.label()),
                        theme::regular(13.0),
                        t.text,
                        (row.width() - 34.0 - reserve).max(0.0),
                    );
                    if on {
                        icons::paint(ui, Rect::from_min_size(body.right_top() + vec2(-24.0, 7.0), vec2(14.0, 14.0)), "check", 14.0, t.accent);
                    }
                    let click = theme::hand(click);
                    let info = tl!(tool.label()).to_string();
                    click.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, info.clone()));
                    if click.clicked() {
                        app.execute(tool.command());
                        ui.close();
                    }
                }
            });
        }
    }
    tool_row_tail(ui, app, cell, t, popup_align);
}

/// Extra controls rendered below/beside the buttons when a comment or fill tool is active.
fn active_tool_controls(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens, vertical: bool) {
    let current_fill = match app.quick_tool {
        QuickTool::Fill(f) => Some(f),
        _ => None,
    };
    if matches!(current_fill, Some(crate::fill_sign::FillTool::Text | crate::fill_sign::FillTool::Date)) {
        // Labelled in both layouts: the strip has room, and a bare number is not self-explaining.
        ui.label(tl!("Font size:"));
        let size = ui.add(
            egui::DragValue::new(&mut app.fill_text_size)
                .range(crate::fill_sign::MIN_TEXT_SIZE..=crate::fill_sign::MAX_TEXT_SIZE)
                .speed(0.25)
                .suffix(" pt"),
        );
        if ((size.changed() && !size.dragged()) || size.drag_stopped())
            && let Some((index, id)) = app.active_ids()
            && let Some((page, annotation)) = app.views.get(index).and_then(|view| view.comments.selected)
            && app.session.get(id).is_some_and(|doc| {
                doc.allows_annotation()
                    && doc
                        .info
                        .annotations
                        .iter()
                        .any(|a| a.page == page && a.index == annotation && a.fill_sign && a.subtype == "FreeText" && !a.locked)
            })
        {
            app.apply_edit(Edit::FontSizeAnnotation { page, index: annotation, size: app.fill_text_size });
        }
        ui.menu_button(tl!("Colour"), |ui| {
            if let Some(color) = comments::swatch_grid(ui, Some(app.fill_text_color)) {
                app.fill_text_color = color;
                ui.close();
            }
        });
    }
    if let QuickTool::Comment(tool) = app.quick_tool {
        let (r, _) = if vertical {
            ui.allocate_exact_size(vec2(crate::canvas::bars::QUICK_BAR_WIDTH, 9.0), Sense::hover())
        } else {
            // The strip is exactly one button tall; the floating bar keeps its own width.
            ui.allocate_exact_size(vec2(9.0, ui.available_height().max(9.0)), Sense::hover())
        };
        if vertical {
            ui.painter().hline(r.x_range().shrink(6.0), r.center().y, Stroke::new(1.0, t.divider));
        } else {
            ui.painter().vline(r.center().x, r.y_range().shrink(6.0), Stroke::new(1.0, t.divider));
        }
        if let Some(change) = comments::quick_bar_controls(ui, tool, &mut app.comment_prefs)
            && let Some((index, id)) = app.active_ids()
            && let Some((page, annotation)) = app.views.get(index).and_then(|view| view.comments.selected)
            && app.session.get(id).is_some_and(|doc| {
                doc.allows_annotation()
                    && doc
                        .info
                        .annotations
                        .iter()
                        .any(|a| a.page == page && a.index == annotation && !a.is_reply() && !a.locked && comments::made_by(a, tool))
            })
        {
            app.apply_edit(Edit::StyleAnnotation {
                page,
                index: annotation,
                color: change.color,
                opacity: change.opacity,
                width: change.width,
                endings: None,
            });
        }
    }
}

/// Render a modern iOS-style pill toggle switch.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, label: &str, t: &Tokens) -> egui::Response {
    let width = 34.0;
    let height = 18.0;
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, ui.is_enabled(), *on, label));

    if ui.is_rect_visible(rect) {
        let radius = height / 2.0;
        let bg_color = if *on { t.accent } else { t.border };
        ui.painter().rect(rect, CornerRadius::same(radius as u8), bg_color, Stroke::NONE, egui::StrokeKind::Inside);
        let circle_radius = radius - 2.5;
        let circle_x = if *on { rect.max.x - radius } else { rect.min.x + radius };
        ui.painter().circle_filled(egui::pos2(circle_x, rect.center().y), circle_radius, Color32::WHITE);
    }
    response
}

/// Render the dock placement choice: [ Float | Left | Right | Top | Bottom ], scrolling sideways
/// when the picker is narrower than the row.
fn segmented_dock_picker(ui: &mut egui::Ui, bar: &mut Hotbar, t: &Tokens) {
    let options = [
        (None, tl!("Float")),
        (Some("left"), tl!("Left")),
        (Some("right"), tl!("Right")),
        (Some("top"), tl!("Top")),
        (Some("bottom"), tl!("Bottom")),
    ];
    let mut side = options.iter().map(|(side, _)| *side).find(|side| *side == bar.dock.as_deref()).flatten();
    let changed = egui::ScrollArea::horizontal()
        .id_salt("hotbar-dock")
        .auto_shrink([false, true])
        .show(ui, |ui| crate::dialogs::preferences::segmented_pill_choice(ui, t, &mut side, &options))
        .inner;
    if changed {
        // `place` accepts every side listed above.
        let _ = bar.place(side.unwrap_or("float"));
    }
}

/// Draw the unified floating or side-docked toolbar, combining quick actions, core tools, and custom shortcuts.
pub(crate) fn show(app: &mut PdfCraftApp, area: Rect, page_left: Option<f32>, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let mut bar = std::mem::take(&mut app.hotbar);
    bar.normalize();
    let t = Tokens::get(&ctx);
    let id = egui::Id::unique("quick-bar");
    let full_side = bar.full_side && bar.dock.is_some();
    let bounds = if full_side { area } else { area.shrink(10.0) };
    if let Some(side) = bar.dock.clone() {
        let _ = bar.place(&side);
    }
    let cell = bar.button_size;
    let popup_align = popup_align_for(bar.dock.as_deref(), bar.vertical);
    let mut size = ctx.data_mut(|data| data.get_temp::<egui::Vec2>(id)).unwrap_or(vec2(crate::canvas::bars::QUICK_BAR_WIDTH, 220.0));
    if full_side {
        size = if bar.vertical { vec2(cell + 12.0, bounds.height()) } else { vec2(bounds.width(), cell + 12.0) };
    }
    let travel = (bounds.size() - size).max(vec2(0.0, 0.0));
    let default_x = crate::canvas::bars::quick_bar_offset(page_left.map_or(0.0, |left| left - area.left()), size.x);
    let pos = if full_side {
        bounds.min + vec2(travel.x * bar.position[0], travel.y * bar.position[1])
    } else if bar.dock.is_none() && !bar.placed {
        area.left_top() + vec2(default_x, crate::canvas::bars::QUICK_BAR_EDGE)
    } else {
        bounds.min + vec2(travel.x * bar.position[0], travel.y * bar.position[1])
    };
    let mut run = None;
    let was_picker_open = bar.picker_open;
    let mut picker_open = bar.picker_open;
    let mut settings_clicked = false;
    let shown = egui::Area::new(id).order(egui::Order::Middle).fixed_pos(pos).constrain_to(bounds).show(&ctx, |ui| {
        let frame = if full_side {
            egui::Frame::NONE.fill(t.chrome).corner_radius(egui::CornerRadius::ZERO).inner_margin(6)
        } else {
            egui::Frame::new()
                .fill(t.card)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .shadow(egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if t.dark() { 100 } else { 28 }) })
                .inner_margin(6)
        };
        let frame_resp = frame.show(ui, |ui| {
            if full_side {
                ui.set_min_size((size - vec2(12.0, 12.0)).max(vec2(0.0, 0.0)));
            }
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            if bar.vertical {
                ui.set_width(cell);
            } else {
                ui.set_height(cell);
            }
            let mut remove = None;
            if full_side {
                if bar.vertical {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        settings_clicked |= settings_button(ui, cell, &mut picker_open);
                        ui.add_space(4.0);
                        let remaining = ui.available_height().max(cell);
                        egui::ScrollArea::vertical().max_height(remaining).show(ui, |ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                core_tool_buttons(ui, app, cell, popup_align, &t, false);
                                active_tool_controls(ui, app, &t, true);
                                if !bar.tools.is_empty() {
                                    ui.separator();
                                    tool_buttons(ui, app, &bar.tools, cell, &mut run, &mut remove);
                                }
                            });
                        });
                    });
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        settings_clicked |= settings_button(ui, cell, &mut picker_open);
                        ui.add_space(4.0);
                        let remaining = ui.available_width().max(cell);
                        egui::ScrollArea::horizontal().max_height(cell).max_width(remaining).show(ui, |ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                core_tool_buttons(ui, app, cell, popup_align, &t, false);
                                active_tool_controls(ui, app, &t, false);
                                if !bar.tools.is_empty() {
                                    ui.separator();
                                    tool_buttons(ui, app, &bar.tools, cell, &mut run, &mut remove);
                                }
                            });
                        });
                    });
                }
            } else {
                let layout =
                    if bar.vertical { egui::Layout::top_down(egui::Align::Center) } else { egui::Layout::left_to_right(egui::Align::Center) };
                ui.with_layout(layout, |ui| {
                    let grip_size = if bar.vertical { vec2(cell, 18.0) } else { vec2(18.0, cell) };
                    let (grip_rect, grip) = ui.allocate_exact_size(grip_size, Sense::drag());
                    for x in [-3.0, 3.0] {
                        for y in [-6.0, 0.0, 6.0] {
                            let offset = if bar.vertical { vec2(y, x) } else { vec2(x, y) };
                            ui.painter().circle_filled(grip_rect.center() + offset, 1.5, t.text_faint);
                        }
                    }
                    let drag_label = tl!("Drag hotbar");
                    let grip = grip.on_hover_text(drag_label).on_hover_cursor(egui::CursorIcon::Grab);
                    grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, drag_label));
                    if grip.dragged() && bar.dock.is_none() {
                        let delta = grip.drag_delta();
                        if !bar.placed {
                            let cur_x = (pos.x - bounds.min.x).max(0.0);
                            let cur_y = (pos.y - bounds.min.y).max(0.0);
                            bar.position[0] = (cur_x / travel.x.max(1.0)).clamp(0.0, 1.0);
                            bar.position[1] = (cur_y / travel.y.max(1.0)).clamp(0.0, 1.0);
                        }
                        bar.position[0] = (bar.position[0] + delta.x / travel.x.max(1.0)).clamp(0.0, 1.0);
                        bar.position[1] = (bar.position[1] + delta.y / travel.y.max(1.0)).clamp(0.0, 1.0);
                        bar.placed = true;
                    }
                    if bar.vertical {
                        let fixed_overhead = grip_size.y + cell + 24.0;
                        let max_tools_length = (bounds.height() - fixed_overhead).max(cell);
                        egui::ScrollArea::vertical().max_height(max_tools_length).show(ui, |ui| {
                            ui.with_layout(layout, |ui| {
                                core_tool_buttons(ui, app, cell, popup_align, &t, false);
                                active_tool_controls(ui, app, &t, true);
                                if !bar.tools.is_empty() {
                                    ui.separator();
                                    tool_buttons(ui, app, &bar.tools, cell, &mut run, &mut remove);
                                }
                            });
                        });
                        settings_clicked |= settings_button(ui, cell, &mut picker_open);
                    } else {
                        core_tool_buttons(ui, app, cell, popup_align, &t, false);
                        active_tool_controls(ui, app, &t, false);
                        if !bar.tools.is_empty() {
                            ui.separator();
                            let scroll_max = (bounds.width() - cell - 50.0).max(cell);
                            egui::ScrollArea::horizontal().max_height(cell).max_width(scroll_max).show(ui, |ui| {
                                ui.with_layout(layout, |ui| {
                                    tool_buttons(ui, app, &bar.tools, cell, &mut run, &mut remove);
                                });
                            });
                        }
                        settings_clicked |= settings_button(ui, cell, &mut picker_open);
                    }
                });
            }
            if let Some(index) = remove {
                bar.tools.remove(index);
            }
        });
        if full_side {
            let r = frame_resp.response.rect;
            let divider = egui::Stroke::new(1.0, t.divider);
            match bar.dock.as_deref() {
                Some("left") => {
                    ui.painter().vline(r.right() - 0.5, r.top()..=r.bottom(), divider);
                }
                Some("right") => {
                    ui.painter().vline(r.left() + 0.5, r.top()..=r.bottom(), divider);
                }
                Some("top") => {
                    ui.painter().hline(r.left()..=r.right(), r.bottom() - 0.5, divider);
                }
                Some("bottom") => {
                    ui.painter().hline(r.left()..=r.right(), r.top() + 0.5, divider);
                }
                _ => {}
            }
        }
    });
    let bar_rect = shown.response.rect;
    picker(&ctx, &t, &mut bar, &mut picker_open, PickerAnchor { rect: bar_rect, was_open: was_picker_open, settings_clicked }, true);
    ctx.data_mut(|data| {
        data.insert_temp(id, shown.response.rect.size());
        data.insert_temp(egui::Id::unique("custom-hotbar"), shown.response.rect.size());
    });
    bar.picker_open = picker_open;
    app.hotbar = bar;
    if let Some(id) = run {
        app.execute(id);
    }
}

/// How the annotation row lays out for a given width: one row when everything fits, otherwise
/// two with the overflow on the second. Never more than two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StripLayout {
    pub rows: u8,
    /// Tools drawn on the first row; the rest go to the second.
    pub split: usize,
}

/// Measure what the row holds. The reading controls and the annotation buttons are fixed-size, so
/// this is exact enough to decide whether the tools wrap without a layout pass, which lets the
/// reserved height change in the same frame.
/// On macOS the traffic lights sit at the top-left of the title bar; the annotation row starts
/// under them, so give the first row the same left inset as the document row.
fn mac_inset(app: &PdfCraftApp) -> f32 {
    if cfg!(target_os = "macos") && app.integrated_titlebar && !app.full_screen { (crate::chrome::MAC_CONTENT_LEFT - 10.0).max(0.0) } else { 0.0 }
}

pub(crate) fn strip_layout(app: &PdfCraftApp, width: f32) -> StripLayout {
    if !app.strip_rows_auto {
        if app.strip_rows == 2 {
            return StripLayout { rows: 2, split: app.strip_tools.len().div_ceil(2) };
        } else {
            return StripLayout { rows: 1, split: app.strip_tools.len() };
        }
    }

    let cell = if app.hotbar.button_size.is_finite() { app.hotbar.button_size.clamp(22.0, 64.0) } else { 32.0 };
    let tiny = cell.min(26.0);
    let reading = 10.0 * (tiny + 2.0) + 160.0;
    let has_controls = matches!(app.quick_tool, QuickTool::Comment(_))
        || matches!(app.quick_tool, QuickTool::Fill(crate::fill_sign::FillTool::Text | crate::fill_sign::FillTool::Date));
    let left_bar = 4.0 * (cell + 2.0) + mac_inset(app) + 24.0 + if has_controls { 180.0 } else { 0.0 };
    let frame = left_bar + reading;
    let select_hand = 2.0 * (cell + 2.0) + 8.0;
    let tool_width = |id: &str| cell + 2.0 + if id == "comment.ink" || id == "comment.square" { 8.0 } else { 0.0 };
    let tools_width: f32 = app.strip_tools.iter().map(|id| tool_width(id)).sum();
    let edit_text = app.mode == crate::Mode::Edit || app.quick_tool == QuickTool::EditText;
    let tail = 8.0 + (cell + 2.0) + if edit_text { cell + 2.0 } else { 0.0 };
    let customize = 8.0 + (cell + 2.0);
    let one_row = frame + select_hand + tools_width + tail + customize + 8.0;
    if one_row <= width {
        return StripLayout { rows: 1, split: app.strip_tools.len() };
    }
    // Automatic: fill the first row with the tools that fit, and wrap the rest to the second.
    let available_for_tools = (width - frame - select_hand - tail - customize).max(0.0);
    let mut used = 0.0;
    let mut split = 0;
    for id in &app.strip_tools {
        let w = tool_width(id);
        if used + w > available_for_tools {
            break;
        }
        used += w;
        split += 1;
    }
    StripLayout { rows: 2, split }
}

/// The height the annotation row takes from the document area: its button rows plus the frame's
/// margins.
pub(crate) fn strip_height(app: &PdfCraftApp, width: f32) -> f32 {
    let cell = if app.hotbar.button_size.is_finite() { app.hotbar.button_size.clamp(22.0, 64.0) } else { 32.0 };
    let rows = f32::from(strip_layout(app, width).rows);
    cell * rows + (rows - 1.0) * 2.0 + 12.0
}

/// What one annotation row holds.
struct RowSpec<'a> {
    tools: &'a [String],
    select_hand: bool,
    tail: bool,
    customize: bool,
}

/// The buttons of one annotation row: Select/Hand, the reader's short list, and optionally the
/// Edit/Fill & Sign tail and the Customize control.
fn strip_buttons(ui: &mut egui::Ui, app: &mut PdfCraftApp, cell: f32, t: &Tokens, spec: RowSpec<'_>) {
    let RowSpec { tools, select_hand, tail, customize } = spec;
    if select_hand {
        if icons::button(ui, "mouse-pointer-2", cell, app.quick_tool == QuickTool::Select, tl!("Select (V)")).clicked() {
            app.quick_tool = QuickTool::Select;
        }
        if icons::button(ui, "hand", cell, app.quick_tool == QuickTool::Hand, tl!("Hand (H)")).clicked() {
            app.quick_tool = QuickTool::Hand;
        }
        ui.separator();
    }
    for id in tools {
        if id == "view.rotate" {
            let icon = if app.rotate_clockwise { "rotate-cw" } else { "rotate-ccw" };
            let tip = if app.rotate_clockwise { tl!("Rotate view clockwise") } else { tl!("Rotate view counterclockwise") };
            let resp = icons::button(ui, icon, cell, false, tip);
            let mut do_rotate = None;
            if resp.clicked() {
                do_rotate = Some(app.rotate_clockwise);
            }
            resp.context_menu(|ui| {
                if ui.button(tl!("Rotate View Right")).clicked() {
                    app.rotate_clockwise = true;
                    do_rotate = Some(true);
                    ui.close();
                }
                if ui.button(tl!("Rotate View Left")).clicked() {
                    app.rotate_clockwise = false;
                    do_rotate = Some(false);
                    ui.close();
                }
            });
            if let Some(cw) = do_rotate
                && let Some(i) = app.active
            {
                app.views[i].rotate_view(cw);
            }
            continue;
        }
        let Some(spec) = pdfcraft_engine::commands::command(id) else { continue };
        if id == "comment.ink" || id == "comment.square" {
            ui.separator();
        }
        let active = comments::CommentTool::from_command(id).is_some_and(|tool| app.quick_tool == QuickTool::Comment(tool));
        if icons::button(ui, spec.icon, cell, active, tl!(spec.label)).clicked() {
            app.execute(id);
        }
    }
    if customize {
        ui.separator();
        if icons::button(ui, "sliders-horizontal", cell, false, tl!("Customize toolbar")).clicked() {
            app.drawer_open = true;
            app.customizing_toolbar = true;
        }
    }
    if tail {
        tool_row_tail(ui, app, cell, t, egui::RectAlign::BOTTOM_START);
    }
}

/// Draw the annotation row: one or two full-width rows at the top of the document area, or docked
/// at the bottom, left, or right, with the tools drawer's hamburger, the reader's own tools, the
/// active tool's controls, and the compact reading controls at the right (or bottom) of the row.
pub(crate) fn strip(app: &mut PdfCraftApp, ctx: &egui::Context, rect: Rect) {
    let t = Tokens::get(ctx);
    let mut bar = std::mem::take(&mut app.hotbar);
    bar.normalize();
    let cell = bar.button_size.clamp(22.0, 64.0);
    app.hotbar = bar;
    let dock = app.hotbar.dock.clone().unwrap_or_else(|| "top".to_owned());
    let is_vertical = dock == "left" || dock == "right";

    if is_vertical {
        let shown = egui::Area::new(egui::Id::unique("annotation-strip"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_width(rect.width());
                ui.set_max_height(rect.height());
                ui.set_clip_rect(rect);
                let frame = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 4, right: 4, top: 8, bottom: 8 }).show(ui, |ui| {
                    ui.set_width(rect.width() - 8.0);
                    ui.set_height(rect.height() - 16.0);
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    let tools = app.strip_tools.clone();
                    let cols = app.strip_rows_shown;
                    strip_vertical_content(app, ui, cell, cell.min(26.0), &t, &tools, cols);
                });
                let bar_rect = frame.response.rect;
                if dock == "left" {
                    ui.painter().vline(bar_rect.right() - 0.5, bar_rect.y_range(), Stroke::new(1.0, t.divider));
                } else {
                    ui.painter().vline(bar_rect.left() + 0.5, bar_rect.y_range(), Stroke::new(1.0, t.divider));
                }
                bar_rect
            })
            .inner;
        app.strip_rect = Some(shown);
        return;
    }

    let layout = strip_layout(app, rect.width());
    app.strip_rows_shown = layout.rows;
    let rows = layout.rows;
    let total = f32::from(rows) * cell + f32::from(rows - 1) * 2.0;
    let tiny = cell.min(26.0);
    let shown = egui::Area::new(egui::Id::unique("annotation-strip"))
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_width(rect.width());
            ui.set_max_height(rect.height());
            ui.set_clip_rect(rect);
            let frame = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 }).show(ui, |ui| {
                ui.set_height(total);
                ui.set_max_height(total);
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                let tools = app.strip_tools.clone();
                let split = layout.split.min(tools.len());
                let has_controls = matches!(app.quick_tool, QuickTool::Comment(_))
                    || matches!(app.quick_tool, QuickTool::Fill(crate::fill_sign::FillTool::Text | crate::fill_sign::FillTool::Date));
                // First row: the drawer, the active tool's controls, then the tools and the
                // reading controls at the right.
                ui.horizontal(|ui| {
                    ui.set_height(cell);
                    ui.add_space(mac_inset(app));
                    if icons::button(ui, "align-justify", cell, app.drawer_open, tl!("Tools")).clicked() {
                        app.drawer_open = !app.drawer_open;
                        app.customizing_toolbar = false;
                    }
                    panels_button(app, ui, cell);
                    if icons::button(ui, "undo-2", cell, false, tl!("Undo (⌘Z)")).clicked() {
                        app.undo();
                    }
                    if icons::button(ui, "redo-2", cell, false, tl!("Redo (⇧⌘Z)")).clicked() {
                        app.redo();
                    }
                    if has_controls {
                        active_tool_controls(ui, app, &t, false);
                        ui.separator();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        reading_controls(app, ui, tiny);
                        ui.separator();
                        let remaining = ui.available_width().max(cell);
                        egui::ScrollArea::horizontal().id_salt("annotation-strip-1").max_height(cell).max_width(remaining).show(ui, |ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                strip_buttons(
                                    ui,
                                    app,
                                    cell,
                                    &t,
                                    RowSpec { tools: &tools[..split], select_hand: true, tail: rows == 1, customize: rows == 1 },
                                );
                            });
                        });
                    });
                });
                if rows == 2 {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.set_height(cell);
                        let remaining = ui.available_width().max(cell);
                        egui::ScrollArea::horizontal().id_salt("annotation-strip-2").max_height(cell).max_width(remaining).show(ui, |ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                strip_buttons(ui, app, cell, &t, RowSpec { tools: &tools[split..], select_hand: false, tail: true, customize: true });
                            });
                        });
                    });
                }
            });
            let bar_rect = frame.response.rect;
            if dock == "bottom" {
                ui.painter().hline(bar_rect.x_range(), bar_rect.top() + 0.5, Stroke::new(1.0, t.divider));
            } else {
                ui.painter().hline(bar_rect.x_range(), bar_rect.bottom() - 0.5, Stroke::new(1.0, t.divider));
            }
            bar_rect
        })
        .inner;
    app.strip_rect = Some(shown);
}

enum StripItem {
    Tool(String),
    Customize,
    EditText,
    FillSign,
}

fn render_tool_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, id: &str) {
    if id == "view.rotate" {
        let icon = if app.rotate_clockwise { "rotate-cw" } else { "rotate-ccw" };
        let tip = if app.rotate_clockwise { tl!("Rotate view clockwise") } else { tl!("Rotate view counterclockwise") };
        let resp = icons::button(ui, icon, cell, false, tip);
        let mut do_rotate = None;
        if resp.clicked() {
            do_rotate = Some(app.rotate_clockwise);
        }
        resp.context_menu(|ui| {
            if ui.button(tl!("Rotate View Right")).clicked() {
                app.rotate_clockwise = true;
                do_rotate = Some(true);
                ui.close();
            }
            if ui.button(tl!("Rotate View Left")).clicked() {
                app.rotate_clockwise = false;
                do_rotate = Some(false);
                ui.close();
            }
        });
        if let Some(cw) = do_rotate
            && let Some(i) = app.active
        {
            app.views[i].rotate_view(cw);
        }
        return;
    }
    let Some(spec) = pdfcraft_engine::commands::command(id) else { return };
    let active = comments::CommentTool::from_command(id).is_some_and(|tool| app.quick_tool == QuickTool::Comment(tool));
    if icons::button(ui, spec.icon, cell, active, tl!(spec.label)).clicked() {
        app.execute(id);
    }
}

fn render_strip_item(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, t: &Tokens, item: &StripItem, popup_align: egui::RectAlign) {
    match item {
        StripItem::Tool(id) => render_tool_button(app, ui, cell, id),
        StripItem::Customize => {
            if icons::button(ui, "sliders-horizontal", cell, false, tl!("Customize toolbar")).clicked() {
                app.drawer_open = true;
                app.customizing_toolbar = true;
            }
        }
        StripItem::EditText => render_edit_text_button(app, ui, cell),
        StripItem::FillSign => render_fill_sign_button(app, ui, cell, t, popup_align),
    }
}

fn strip_vertical_tools(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, t: &Tokens, tools: &[String], cols: u8, popup_align: egui::RectAlign) {
    let mut items: Vec<StripItem> = tools.iter().cloned().map(StripItem::Tool).collect();
    items.push(StripItem::Customize);
    if app.mode == crate::Mode::Edit || app.quick_tool == QuickTool::EditText {
        items.push(StripItem::EditText);
    }
    items.push(StripItem::FillSign);

    if cols == 2 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            if icons::button(ui, "mouse-pointer-2", cell, app.quick_tool == QuickTool::Select, tl!("Select (V)")).clicked() {
                app.quick_tool = QuickTool::Select;
            }
            if icons::button(ui, "hand", cell, app.quick_tool == QuickTool::Hand, tl!("Hand (H)")).clicked() {
                app.quick_tool = QuickTool::Hand;
            }
        });
        ui.separator();
        for chunk in items.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                for item in chunk {
                    render_strip_item(app, ui, cell, t, item, popup_align);
                }
            });
        }
    } else {
        if icons::button(ui, "mouse-pointer-2", cell, app.quick_tool == QuickTool::Select, tl!("Select (V)")).clicked() {
            app.quick_tool = QuickTool::Select;
        }
        if icons::button(ui, "hand", cell, app.quick_tool == QuickTool::Hand, tl!("Hand (H)")).clicked() {
            app.quick_tool = QuickTool::Hand;
        }
        ui.separator();
        for item in &items {
            render_strip_item(app, ui, cell, t, item, popup_align);
        }
    }
}

fn strip_vertical_content(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, _tiny: f32, t: &Tokens, tools: &[String], cols: u8) {
    ui.vertical_centered(|ui| {
        if cols == 2 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                if icons::button(ui, "align-justify", cell, app.drawer_open, tl!("Tools")).clicked() {
                    app.drawer_open = !app.drawer_open;
                    app.customizing_toolbar = false;
                }
                panels_button(app, ui, cell);
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                if icons::button(ui, "undo-2", cell, false, tl!("Undo (⌘Z)")).clicked() {
                    app.undo();
                }
                if icons::button(ui, "redo-2", cell, false, tl!("Redo (⇧⌘Z)")).clicked() {
                    app.redo();
                }
            });
        } else {
            if icons::button(ui, "align-justify", cell, app.drawer_open, tl!("Tools")).clicked() {
                app.drawer_open = !app.drawer_open;
                app.customizing_toolbar = false;
            }
            panels_button(app, ui, cell);
            if icons::button(ui, "undo-2", cell, false, tl!("Undo (⌘Z)")).clicked() {
                app.undo();
            }
            if icons::button(ui, "redo-2", cell, false, tl!("Redo (⇧⌘Z)")).clicked() {
                app.redo();
            }
        }

        let has_controls = matches!(app.quick_tool, QuickTool::Comment(_))
            || matches!(app.quick_tool, QuickTool::Fill(crate::fill_sign::FillTool::Text | crate::fill_sign::FillTool::Date));
        if has_controls {
            ui.separator();
            active_tool_controls(ui, app, t, true);
        }

        ui.separator();
        let reading_h = if cols == 2 { 5.0 * (cell + 2.0) + 38.0 } else { 10.0 * (cell + 2.0) + 38.0 };
        let scroll_h = (ui.available_height() - reading_h - 12.0).max(cell);
        let popup_align = popup_align_for(app.hotbar.dock.as_deref(), true);
        egui::ScrollArea::vertical().id_salt("annotation-strip-v").max_height(scroll_h).auto_shrink([false, false]).show(ui, |ui| {
            strip_vertical_tools(app, ui, cell, t, tools, cols, popup_align);
        });
        ui.separator();
        reading_controls_vertical(app, ui, cell, cols, t);
    });
}

fn dock_position_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let resp = icons::button(ui, "move", cell, false, tl!("Toolbar position"));
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    egui::Popup::menu(&resp).align(align).show(|ui| {
        ui.set_min_width(140.0);
        let options = [(None, "Top"), (Some("bottom"), "Bottom"), (Some("left"), "Left"), (Some("right"), "Right")];
        for (dock, label) in options {
            let active = app.hotbar.dock.as_deref() == dock;
            if ui.selectable_label(active, tl!(label)).clicked() {
                app.hotbar.dock = dock.map(str::to_owned);
                if let Some(side) = dock {
                    let _ = app.hotbar.place(side);
                }
                ui.close();
            }
        }
    });
}

fn reading_controls_vertical(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32, cols: u8, t: &Tokens) {
    if cols == 2 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            dock_position_button(app, ui, cell);
            if icons::button(ui, "maximize", cell, false, tl!("Full Screen (⌘L)")).clicked() {
                app.execute("view.full_screen");
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            if icons::button(ui, "settings", cell, false, tl!("Preferences")).clicked() {
                app.execute("app.preferences");
            }
            hscroll_lock_button(app, ui, cell);
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            view_button(app, ui, cell);
            zoom_menu_button(app, ui, cell);
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            zoom_step_buttons(app, ui, cell);
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            page_nav_buttons(app, ui, cell);
        });
        let indicator_width = 2.0 * cell + 2.0;
        page_indicator_vertical(app, ui, t, indicator_width, cell);
    } else {
        dock_position_button(app, ui, cell);
        if icons::button(ui, "maximize", cell, false, tl!("Full Screen (⌘L)")).clicked() {
            app.execute("view.full_screen");
        }
        if icons::button(ui, "settings", cell, false, tl!("Preferences")).clicked() {
            app.execute("app.preferences");
        }
        hscroll_lock_button(app, ui, cell);
        view_button(app, ui, cell);
        zoom_menu_button(app, ui, cell);
        if icons::button(ui, "zoom-out", cell, false, tl!("Zoom out")).clicked()
            && let Some(i) = app.active
        {
            app.views[i].zoom_step(false);
        }
        if icons::button(ui, "zoom-in", cell, false, tl!("Zoom in")).clicked()
            && let Some(i) = app.active
        {
            app.views[i].zoom_step(true);
        }
        if icons::button(ui, "chevron-left", cell, false, tl!("Previous page")).clicked()
            && let Some(i) = app.active
        {
            app.views[i].step_page(false);
        }
        if icons::button(ui, "chevron-right", cell, false, tl!("Next page")).clicked()
            && let Some(i) = app.active
        {
            app.views[i].step_page(true);
        }
        page_indicator_vertical(app, ui, t, cell, cell);
    }
}

/// The reading controls at the right of the annotation row, added right to left: settings, panels,
/// page display, zoom and page navigation. Everything the old navigation rail held lives here.
fn reading_controls(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing.x = 0.0;
    dock_position_button(app, ui, cell);
    if icons::button(ui, "maximize", cell, false, tl!("Full Screen (⌘L)")).clicked() {
        app.execute("view.full_screen");
    }
    if icons::button(ui, "settings", cell, false, tl!("Preferences")).clicked() {
        app.execute("app.preferences");
    }
    view_button(app, ui, cell);
    zoom_controls(app, ui, cell);
    hscroll_lock_button(app, ui, cell);
    page_controls(app, ui, cell);
    ui.spacing_mut().item_spacing = spacing;
}

/// The panels the old rail switched between, as one menu. Choosing one docks its inspector.
fn panels_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let resp = icons::button(ui, "panel-left", cell, app.right.is_some(), tl!("Panels"));
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    egui::Popup::menu(&resp).align(align).show(|ui| {
        ui.set_min_width(190.0);
        let entries = [
            (crate::RightPanel::Comments, "Comments"),
            (crate::RightPanel::Bookmarks, "Bookmarks"),
            (crate::RightPanel::Pages, "Page thumbnails"),
            (crate::RightPanel::Fields, "Form fields"),
            (crate::RightPanel::Layers, "Layers"),
            (crate::RightPanel::Attachments, "Attachments"),
            (crate::RightPanel::Signatures, "Signatures"),
            (crate::RightPanel::Tags, "Tags"),
        ];
        for (panel, label) in entries {
            if ui.button(tl!(label)).clicked() {
                app.choose_right_panel(Some(panel));
                ui.close();
            }
        }
        if app.right.is_some() {
            ui.separator();
            if ui.button(tl!("Close panel")).clicked() {
                app.choose_right_panel(None);
                ui.close();
            }
        }
    });
}

/// Keep the current horizontal page position while moving between pages of different widths.
fn hscroll_lock_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    let locked = app.views[index].hscroll_lock;
    let tip = if locked { tl!("Unlock horizontal position") } else { tl!("Lock horizontal position") };
    if crate::icons::button(ui, if locked { "lock" } else { "lock-open" }, cell, locked, tip).clicked() {
        app.views[index].hscroll_lock = !locked;
    }
}

/// Page display, fit, rotation and history in one menu, the controls the rail kept below its page
/// box.
fn view_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    let resp = icons::button(ui, "layout-grid", cell, false, tl!("Page display and rotation"));
    let view = &mut app.views[index];
    let mut toggle_gaps = false;
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    egui::Popup::menu(&resp).align(align).show(|ui| {
        ui.set_min_width(210.0);
        let is_single_page = view.layout == crate::canvas::PageLayout::Single && !view.continuous;
        let is_single_scroll = view.layout == crate::canvas::PageLayout::Single && view.continuous;
        let is_two_page = view.layout == crate::canvas::PageLayout::TwoUp && !view.continuous;
        let is_two_scroll = view.layout == crate::canvas::PageLayout::TwoUp && view.continuous;

        if ui.add(crate::widgets::radio(is_single_page, tl!("Single page view"))).clicked() {
            view.layout = crate::canvas::PageLayout::Single;
            view.continuous = false;
            ui.close();
        }
        if ui.add(crate::widgets::radio(is_single_scroll, tl!("Single page scrolling"))).clicked() {
            view.layout = crate::canvas::PageLayout::Single;
            view.continuous = true;
            ui.close();
        }
        if ui.add(crate::widgets::radio(is_two_page, tl!("Two-page view"))).clicked() {
            view.layout = crate::canvas::PageLayout::TwoUp;
            view.continuous = false;
            ui.close();
        }
        if ui.add(crate::widgets::radio(is_two_scroll, tl!("Two-page scrolling"))).clicked() {
            view.layout = crate::canvas::PageLayout::TwoUp;
            view.continuous = true;
            ui.close();
        }
        ui.separator();
        let mut cover = view.cover;
        if ui.add_enabled(view.layout == crate::canvas::PageLayout::TwoUp, crate::widgets::checkbox(&mut cover, tl!("Separate cover page"))).changed()
        {
            view.cover = cover;
        }
        let mut gaps = view.show_page_gaps;
        if ui.add(crate::widgets::checkbox(&mut gaps, tl!("Show gaps between pages"))).changed() {
            toggle_gaps = true;
        }
        ui.separator();
        if ui.button(tl!("Rotate view clockwise")).clicked() {
            view.rotate_view(true);
            ui.close();
        }
        if ui.button(tl!("Rotate view counterclockwise")).clicked() {
            view.rotate_view(false);
            ui.close();
        }
        ui.separator();
        let can_back = !view.back.is_empty();
        let can_forward = !view.forward.is_empty();
        let back_tip = format!("{} ({})", tl!("Previous view"), crate::commands::shortcut_label(ui.ctx(), crate::commands::PREV_VIEW));
        if ui.add_enabled(can_back, egui::Button::new(back_tip)).clicked() {
            view.view_history(false);
            ui.close();
        }
        let forward_tip = format!("{} ({})", tl!("Next view"), crate::commands::shortcut_label(ui.ctx(), crate::commands::NEXT_VIEW));
        if ui.add_enabled(can_forward, egui::Button::new(forward_tip)).clicked() {
            view.view_history(true);
            ui.close();
        }
    });
    if toggle_gaps {
        app.execute("view.page_gaps");
    }
}

fn zoom_popup_content(view: &mut crate::DocView, ui: &mut egui::Ui) {
    // Typing a zoom is precise and compact: it lives in the percentage menu instead of a
    // permanent field in the reading row.
    ui.horizontal(|ui| {
        ui.label(tl!("Zoom"));
        let id = egui::Id::unique(("zoom-input", view.id.0));
        if !ui.memory(|m| m.has_focus(id)) {
            view.zoom_input = format!("{:.0}%", view.zoom * 100.0);
        }
        let mut text = std::mem::take(&mut view.zoom_input);
        let edit = ui.add(egui::TextEdit::singleline(&mut text).id(id).desired_width(72.0).hint_text("100%"));
        edit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, tl!("Zoom")));
        if edit.gained_focus() {
            let mut state = egui::TextEdit::load_state(ui.ctx(), edit.id).unwrap_or_default();
            let len = text.chars().count();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(len))));
            state.store(ui.ctx(), edit.id);
        }
        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let typed = text.trim().trim_end_matches('%').trim().to_owned();
            if let Ok(percent) = typed.parse::<f32>()
                && percent > 0.0
            {
                view.set_zoom((percent / 100.0).clamp(0.1, 64.0));
            }
            ui.close();
        }
        view.zoom_input = text;
    });
    ui.separator();
    if ui.button(tl!("Actual size")).clicked() {
        view.set_zoom(1.0);
        ui.close();
    }
    if ui.button(tl!("Fit to page")).clicked() {
        view.set_fit(crate::canvas::Fit::Page);
        ui.close();
    }
    if ui.button(tl!("Fit to width")).clicked() {
        view.set_fit(crate::canvas::Fit::Width);
        ui.close();
    }
    if ui.button(tl!("Fit to height")).clicked() {
        view.set_fit(crate::canvas::Fit::Height);
        ui.close();
    }
    ui.separator();
    for (label, zoom) in [("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0), ("125%", 1.25), ("150%", 1.5), ("200%", 2.0), ("400%", 4.0)] {
        if ui.button(label).clicked() {
            view.set_zoom(zoom);
            ui.close();
        }
    }
}

/// Zoom out, the percentage (a menu of presets), and zoom in.
fn zoom_controls(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    let view = &mut app.views[index];
    if icons::button(ui, "zoom-in", cell, false, tl!("Zoom in")).clicked() {
        view.zoom_step(true);
    }
    let percent = format!("{:.0}%", view.zoom * 100.0);
    let resp = ui
        .add_sized(
            [38.0, cell],
            egui::Label::new(egui::RichText::new(percent).font(crate::theme::medium(11.0)).color(crate::theme::Tokens::get(ui.ctx()).text))
                .sense(Sense::click()),
        )
        .on_hover_text(tl!("Zoom"))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Zoom")));
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    egui::Popup::menu(&resp).align(align).show(|ui| {
        zoom_popup_content(view, ui);
    });
    if icons::button(ui, "zoom-out", cell, false, tl!("Zoom out")).clicked() {
        view.zoom_step(false);
    }
}

fn zoom_menu_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    let view = &mut app.views[index];
    let percent = format!("{:.0}%", view.zoom * 100.0);
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    let resp = ui
        .add_sized(
            [cell, cell],
            egui::Button::new(egui::RichText::new(percent).font(crate::theme::medium(10.0)).color(crate::theme::Tokens::get(ui.ctx()).text)),
        )
        .on_hover_text(tl!("Zoom presets and fit options"))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Zoom")));
    egui::Popup::menu(&resp).align(align).show(|ui| {
        zoom_popup_content(view, ui);
    });
}

fn zoom_step_buttons(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    if icons::button(ui, "zoom-out", cell, false, tl!("Zoom out")).clicked() {
        app.views[index].zoom_step(false);
    }
    if icons::button(ui, "zoom-in", cell, false, tl!("Zoom in")).clicked() {
        app.views[index].zoom_step(true);
    }
}

fn page_nav_buttons(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    if icons::button(ui, "chevron-left", cell, false, tl!("Previous page")).clicked() {
        app.views[index].step_page(false);
    }
    if icons::button(ui, "chevron-right", cell, false, tl!("Next page")).clicked() {
        app.views[index].step_page(true);
    }
}

fn page_indicator_vertical(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, width: f32, _cell: f32) {
    let Some(index) = app.active else { return };
    let Some(id) = app.views.get(index).map(|v| v.id) else { return };
    let page_count = app.session.get(id).map_or(0, |doc| doc.info.pages.len());
    let labels: Vec<String> = app.session.get(id).map(|doc| doc.info.pages.iter().map(|p| p.label.clone()).collect()).unwrap_or_default();
    let view = &mut app.views[index];
    let cur_label = view.current_page_label(labels.get(view.current).map(String::as_str));
    let text = if page_count > 0 { format!("{cur_label} / {page_count}") } else { cur_label };
    let is_vert = matches!(app.hotbar.dock.as_deref(), Some("left" | "right"));
    let align = popup_align_for(app.hotbar.dock.as_deref(), is_vert);
    let resp = ui
        .add_sized([width, 22.0], egui::Button::new(egui::RichText::new(text).font(crate::theme::regular(10.5)).color(t.text_muted)))
        .on_hover_text(tl!("Current page — click to jump"));

    egui::Popup::menu(&resp).align(align).show(|ui| {
        ui.horizontal(|ui| {
            ui.label(tl!("Go to page:"));
            let input_id = egui::Id::unique(("page-jump-v", view.id.0));
            if !ui.memory(|m| m.has_focus(input_id)) {
                view.page_input = view.current_page_label(labels.get(view.current).map(String::as_str));
            }
            let mut edit_text = std::mem::take(&mut view.page_input);
            let edit = ui.add(egui::TextEdit::singleline(&mut edit_text).id(input_id).desired_width(60.0));
            if edit.gained_focus() {
                let mut state = egui::TextEdit::load_state(ui.ctx(), edit.id).unwrap_or_default();
                let len = edit_text.chars().count();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(len))));
                state.store(ui.ctx(), edit.id);
            }
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let typed = edit_text.clone();
                view.go_to_typed(&typed, &labels);
                ui.close();
            }
            view.page_input = edit_text;
        });
    });
}

/// The current page field, with the previous and next page arrows either side.
fn page_controls(app: &mut PdfCraftApp, ui: &mut egui::Ui, cell: f32) {
    let Some(index) = app.active else { return };
    let Some(id) = app.views.get(index).map(|v| v.id) else { return };
    let page_count = app.session.get(id).map_or(0, |doc| doc.info.pages.len());
    let labels: Vec<String> = app.session.get(id).map(|doc| doc.info.pages.iter().map(|p| p.label.clone()).collect()).unwrap_or_default();
    let view = &mut app.views[index];
    if icons::button(ui, "chevron-right", cell, false, tl!("Next page")).clicked() {
        view.step_page(true);
    }
    if page_count > 0 {
        ui.label(
            egui::RichText::new(format!("/{page_count}")).font(crate::theme::regular(11.0)).color(crate::theme::Tokens::get(ui.ctx()).text_muted),
        );
    }
    let page_input_id = view.page_input_id();
    let focused = ui.memory(|m| m.has_focus(page_input_id));
    if !focused {
        view.page_input = view.current_page_label(labels.get(view.current).map(String::as_str));
    } else if view.page_input_reselect {
        // A reload refreshed the label under the caret; select it so typing replaces the whole label.
        let len = view.page_input.chars().count();
        let mut state = egui::TextEdit::load_state(ui.ctx(), page_input_id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(len))));
        state.store(ui.ctx(), page_input_id);
        view.page_input_reselect = false;
    }
    let edit = egui::TextEdit::singleline(&mut view.page_input)
        .id(page_input_id)
        .align(egui::Align2::CENTER_CENTER)
        .font(crate::theme::medium(12.0))
        .frame(egui::Frame::NONE)
        .margin(egui::vec2(1.0, 3.0));
    let r = ui
        .add_sized([30.0, cell], edit)
        .on_hover_text(tl!("Current page — type a page number or label and press Enter"))
        .accessible_name(tl!("Current page"));
    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let typed = view.page_input.clone();
        view.go_to_typed(&typed, &labels);
        view.page_input = view.current_page_label(labels.get(view.current).map(String::as_str));
    }
    if icons::button(ui, "chevron-left", cell, false, tl!("Previous page")).clicked() {
        view.step_page(false);
    }
}

/// Where the picker hangs from, and the click state that opened it.
struct PickerAnchor {
    rect: Rect,
    was_open: bool,
    settings_clicked: bool,
}

/// The tool picker: toolbar size, how the floating bar is laid out, and every command that can be
/// added to the bar. Opened by the settings button and floating above the page. `floating` hides
/// the layout controls the top strip does not use.
fn picker(ctx: &egui::Context, t: &Tokens, bar: &mut Hotbar, picker_open: &mut bool, anchor: PickerAnchor, floating: bool) {
    let PickerAnchor { rect: bar_rect, was_open: was_picker_open, settings_clicked } = anchor;
    let picker_id = egui::Id::unique("hotbar-picker");
    // A remembered collapsed state can survive disabling the collapse affordance.
    let mut expanded = egui::collapsing_header::CollapsingState::load_with_default_open(ctx, picker_id.with("collapsing"), true);
    expanded.set_open(true);
    expanded.store(ctx);
    let screen = ctx.content_rect().shrink(12.0);
    let margin_x = 14.0;
    let card_margin_total = margin_x * 2.0;
    let card_width = 280.0f32.min((screen.width() - card_margin_total).max(180.0));
    let outer_width = card_width + card_margin_total;
    let picker_x =
        if bar_rect.right() + outer_width + 10.0 < screen.right() { bar_rect.right() + 14.0 } else { bar_rect.left() - outer_width - 14.0 };
    let picker_y = (bar_rect.bottom() + 10.0).clamp(screen.top(), (screen.bottom() - 480.0).max(screen.top()));
    let picker_pos = egui::pos2(picker_x.clamp(screen.left(), (screen.right() - outer_width).max(screen.left())), picker_y);
    let mut close_dialog = false;
    let picker_resp = egui::Window::new(tl!("Hotbar tools"))
        .id(picker_id)
        .order(egui::Order::Foreground)
        .open(picker_open)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_pos(picker_pos)
        .frame(
            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(10))
                .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: Color32::from_black_alpha(if t.dark() { 120 } else { 35 }) })
                .inner_margin(egui::Margin::symmetric(14, 12)),
        )
        .show(ctx, |ui| {
            ui.set_width(card_width);

            // Title bar: "Hotbar Tools" on the left, "X" close button on the right
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tl!("Hotbar tools")).size(14.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button(ui, "x", 16.0, false, tl!("Close menu")).clicked() {
                        close_dialog = true;
                    }
                });
            });

            ui.add_space(10.0);

            // Section 1: Layout & Sizing
            ui.label(egui::RichText::new(tl!("Layout & Sizing")).size(13.0).strong().color(t.text));
            ui.add_space(6.0);

            // Toolbar size slider row
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tl!("Toolbar size")).size(12.5).color(t.text));
                let slider_width = (ui.available_width() - 55.0).clamp(50.0, 140.0);
                ui.spacing_mut().slider_width = slider_width;
                ui.add(egui::Slider::new(&mut bar.button_size, 24.0..=64.0).show_value(false));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{:.0} px", bar.button_size)).size(12.0).color(t.text_muted));
                });
            });

            ui.add_space(8.0);

            if floating {
                // Vertical layout toggle switch row
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(bar.dock.is_none(), |ui| {
                        let label = tl!("Vertical layout");
                        toggle_switch(ui, &mut bar.vertical, label, t);
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(label).size(12.5).color(t.text));
                    });
                });

                ui.add_space(10.0);

                // Stick to a side section with segmented control
                ui.label(egui::RichText::new(tl!("Stick to a side:")).size(12.5).color(t.text));
                ui.add_space(6.0);
                segmented_dock_picker(ui, bar, t);

                ui.add_space(8.0);

                // Full-side strip checkbox
                let strip_hint = tl!("Extend the docked toolbar along the entire side. Choose a side above to enable.");
                ui.add_enabled(bar.dock.is_some(), crate::widgets::checkbox(&mut bar.full_side, tl!("Full-side strip")))
                    .on_hover_text(strip_hint)
                    .on_disabled_hover_text(strip_hint);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            // Section 2: Add Tools to Hotbar
            ui.label(egui::RichText::new(tl!("Add Tools to Hotbar")).size(13.0).strong().color(t.text));
            ui.add_space(8.0);

            // Search bar pill
            let search_frame = egui::Frame::NONE
                .fill(t.field)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(10, 6));
            search_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add(icons::image("search", 14.0, t.text_faint));
                    ui.add(
                        egui::TextEdit::singleline(&mut bar.search)
                            .hint_text(tl!("Search all tools…"))
                            .frame(egui::Frame::NONE)
                            .desired_width(ui.available_width()),
                    );
                });
            });

            ui.add_space(8.0);

            // Scrollable tools list with checkboxes
            let max_list_height = (screen.height() - 340.0).clamp(120.0, 220.0);
            egui::ScrollArea::vertical().max_height(max_list_height).auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;
                for spec in COMMANDS {
                    if !matches_search(spec, &bar.search) {
                        continue;
                    }
                    let mut selected = bar.tools.iter().any(|id| id == spec.id);
                    if ui.add(crate::widgets::checkbox(&mut selected, tl!(spec.label))).changed() {
                        if selected {
                            bar.tools.push(spec.id.to_owned());
                        } else {
                            bar.tools.retain(|id| id != spec.id);
                        }
                    }
                }
            });
        });
    let clicked_outside = was_picker_open
        && !settings_clicked
        && picker_resp.as_ref().is_some_and(|wr| {
            wr.response.clicked_elsewhere()
                || ctx.input(|i| i.pointer.any_click() && !i.pointer.interact_pos().is_some_and(|p| wr.response.rect.contains(p)))
        });
    if clicked_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_dialog = true;
    }
    if close_dialog {
        *picker_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotbar_validation_and_placement() {
        let mut bar = Hotbar::default();
        assert!(bar.tools.is_empty());
        bar.set_tools("comment.ink,comment.freehand_highlight,comment.ink").unwrap();
        assert_eq!(bar.tools.len(), 2);
        assert!(bar.set_tools("does.not.exist").is_err());
        assert_eq!(bar.tools.len(), 2);
        bar.place("right").unwrap();
        assert!(bar.vertical);
        assert_eq!(bar.position, [1.0, 0.5]);
        bar.place("bottom").unwrap();
        assert!(!bar.vertical);
        assert_eq!(bar.position, [0.5, 1.0]);
        assert_eq!(bar.dock.as_deref(), Some("bottom"));
        bar.place("float").unwrap();
        assert!(bar.dock.is_none());
    }

    #[test]
    fn hotbar_preferences_round_trip() {
        let mut app = PdfCraftApp::new();
        app.set_option("hotbar-tools", "comment.freehand_highlight,comment.ink").unwrap();
        app.set_option("hotbar-side", "left").unwrap();
        app.set_option("hotbar-size", "48").unwrap();
        app.set_option("hotbar-full-side", "true").unwrap();
        assert!(app.set_option("hotbar-size", "NaN").is_err());
        let mut restored = PdfCraftApp::new();
        restored.restore(&app.persist());
        assert_eq!(restored.hotbar.tools, app.hotbar.tools);
        assert_eq!(restored.hotbar.position, [0.0, 0.5]);
        assert!(restored.hotbar.vertical);
        assert_eq!(restored.hotbar.button_size, 48.0);
        assert!(restored.hotbar.full_side);
        assert_eq!(restored.hotbar.dock.as_deref(), Some("left"));
    }

    #[test]
    fn hotbar_restore_filters_unknown_tools_and_offscreen_positions() {
        let mut app = PdfCraftApp::new();
        app.restore(r#"{"hotbar":{"tools":["invalid","comment.ink","comment.ink"],"position":[-10,999]}}"#);
        assert_eq!(app.hotbar.tools, ["comment.ink"]);
        assert_eq!(app.hotbar.position, [0.0, 1.0]);
    }

    #[test]
    fn hotbar_restore_moves_legacy_default_into_margin_and_keeps_chosen_spots() {
        let mut app = PdfCraftApp::new();
        app.restore(r#"{"hotbar":{"tools":["comment.ink"],"position":[0.5,0.0],"vertical":false,"button_size":40.0,"dock":null}}"#);
        assert_eq!(app.hotbar.position, [0.0, 0.0]);
        assert!(app.hotbar.vertical);
        assert!(!app.hotbar.placed);
        assert_eq!(app.hotbar.tools, ["comment.ink"]);
        assert_eq!(app.hotbar.button_size, 40.0);
        app.restore(r#"{"hotbar":{"position":[0.0,0.0],"vertical":false,"dock":null}}"#);
        assert_eq!(app.hotbar.position, [0.0, 0.0]);
        assert!(!app.hotbar.vertical);
        assert!(app.hotbar.placed, "a legacy bar dragged to the corner stays there");
        app.restore(r#"{"hotbar":{"position":[0.5,0.0],"vertical":false,"dock":null,"placed":true}}"#);
        assert_eq!(app.hotbar.position, [0.5, 0.0], "current preferences are not migrated");
        assert!(!app.hotbar.vertical);
    }

    #[test]
    fn hotbar_search_matches_translated_and_canonical_labels() {
        if !crate::i18n::needs_catalogs() {
            return;
        }
        let ink = command("comment.ink").unwrap();
        crate::i18n::set_current(crate::i18n::lang_from_tag("fr").unwrap());
        assert_eq!(tl!(ink.label), "Dessiner à main levée");
        assert!(matches_search(ink, "dessiner"), "translated French label must match");
        assert!(matches_search(ink, "freehand"), "canonical English label must still match");
        assert!(matches_search(ink, "comment.ink"), "command ID must still match");
        assert!(!matches_search(ink, "introuvable"));
        crate::i18n::set_current(crate::i18n::Lang::EN);
    }
}
