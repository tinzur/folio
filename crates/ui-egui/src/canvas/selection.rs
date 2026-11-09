use super::{Arc, DocInfo, DocView, Edit, HashMap, PageText, Pos2, Range, Rect, comments, pos2};

/// A text selection on one page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Selection {
    pub(super) page: usize,
    pub(super) span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Span {
    /// Reading order, between two glyph indices.
    Run { anchor: usize, head: usize },
    /// Column select (#740): the glyphs inside a rectangle in page view space, drawn from the
    /// corner the drag started at (`rect[0..2]`) to the pointer (`rect[2..4]`).
    Column { rect: [f32; 4] },
}

impl Selection {
    pub(super) fn run(page: usize, anchor: usize, head: usize) -> Self {
        Self { page, span: Span::Run { anchor, head } }
    }

    fn range(anchor: usize, head: usize) -> Range<usize> {
        anchor.min(head)..anchor.max(head).saturating_add(1)
    }

    /// One rectangle per selected piece of a line, in view space (highlighting, markup quads).
    pub(super) fn rects(&self, text: &PageText) -> Vec<[f32; 4]> {
        match self.span {
            Span::Run { anchor, head } => text.line_rects(Self::range(anchor, head)),
            Span::Column { rect } => text.glyph_rects(&text.glyphs_in(rect)),
        }
    }

    /// The selected text, as Copy puts it on the clipboard.
    pub(super) fn text(&self, text: &PageText) -> String {
        match self.span {
            Span::Run { anchor, head } => text.text_of(Self::range(anchor, head)),
            Span::Column { rect } => text.column_text(&text.glyphs_in(rect)),
        }
    }
}

/// The glyphs `clicks` quick clicks select around glyph `a`: two the word, three the line, more
/// the page.
pub(super) fn click_unit(text: &PageText, clicks: u32, a: usize) -> Option<(usize, usize)> {
    match clicks {
        2 => text.word_at(a),
        3 => text.line_at(a),
        _ => text.glyphs.len().checked_sub(1).map(|last| (0, last)),
    }
}

impl DocView {
    /// The text selection as markup quadrilaterals in user space: (page, quads).
    pub(crate) fn selection_quads(&self, info: &DocInfo) -> Option<(usize, Vec<[f64; 8]>)> {
        let s = self.selection?;
        let text = self.texts.get(&s.page)?;
        let page = info.pages.get(s.page)?;
        let quads: Vec<[f64; 8]> = s.rects(text).into_iter().map(|r| page.view_rect_to_quad(r)).collect();
        (!quads.is_empty()).then_some((s.page, quads))
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selection = None;
        self.actions_dismissed = None;
    }

    /// Select text on `page` from glyph `from` to glyph `to` (tests and automation).
    pub fn select_text(&mut self, page: usize, from: usize, to: usize) {
        self.all_text = false;
        self.selection = Some(Selection::run(page, from, to));
    }

    /// Screen position of the centre of glyph `glyph` on `page`, if that page is on screen and its
    /// text layer is loaded (used by UI tests and automation to aim pointer input).
    pub fn glyph_screen_pos(&self, page: usize, glyph: usize) -> Option<Pos2> {
        let (_, xf) = self.screen_xforms.iter().find(|(p, _)| *p == page)?;
        let g = self.texts.get(&page)?.glyphs.get(glyph)?;
        Some(xf.view_rect(g.rect).center())
    }

    /// Selected text, if any (⌘C).
    /// The selected text: of the pages read so far, after Select All of the document (Copy reads
    /// the others, [`DocView::pages_without_text`]).
    pub fn selected_text(&self) -> Option<String> {
        if self.all_text {
            return Some(self.whole_text(&HashMap::new())).filter(|x| !x.is_empty());
        }
        let s = self.selection?;
        let t = self.texts.get(&s.page)?;
        Some(s.text(t)).filter(|x| !x.is_empty())
    }

    /// Select every page without navigating; shared by Organize and the Pages sidebar.
    pub fn select_all_pages(&mut self) -> bool {
        self.selected = (0..self.page_count).collect();
        self.select_anchor = (self.page_count > 0).then_some(self.current);
        !self.selected.is_empty()
    }

    /// Select all words on the current page, or every page in Organize.
    pub fn select_all(&mut self) -> bool {
        if self.organize {
            return self.select_all_pages();
        }
        // Scrolling continuously, Select All takes the whole document's text (#2639); one page at
        // a time, the page shown.
        // Like the page's, true once some text is selected: the pages whose text isn't read yet are
        // read when the selection is copied.
        if self.continuous && self.page_count > 1 {
            self.selection = None;
            self.all_text = true;
            return self.selected_text().is_some();
        }
        self.all_text = false;
        let page = self.current;
        let Some(t) = self.texts.get(&page) else { return false };
        if t.glyphs.is_empty() {
            return false;
        }
        self.selection = Some(Selection::run(page, 0, t.glyphs.len() - 1));
        true
    }

    /// Pages whose text [`DocView::whole_text`] still needs: not read yet, nor failed.
    pub(crate) fn pages_without_text(&self) -> Vec<usize> {
        (0..self.page_count).filter(|p| !self.texts.contains_key(p) && !self.text_failed.contains(p)).collect()
    }

    /// The whole document's text in page order, one page after another on new lines, from the
    /// pages' text read so far and `more` (pages read for this, keyed by page).
    pub(crate) fn whole_text(&self, more: &HashMap<usize, Arc<PageText>>) -> String {
        let mut out = String::new();
        for p in 0..self.page_count {
            let Some(t) = self.texts.get(&p).or_else(|| more.get(&p)) else { continue };
            let text = t.text_of(0..t.glyphs.len());
            if text.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&text);
        }
        out
    }
}

#[derive(Clone, Copy)]
enum SelectionAction {
    /// Open the selected text with the web action at this index.
    Search(usize),
    Note,
    Markup(comments::CommentTool),
    Replace,
    Copy,
}

/// The bar's icon row. Returns the action clicked this frame.
fn actions_row(ui: &mut egui::Ui, bar: &SelectionBar<'_>) -> Option<SelectionAction> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        let cell = 24.0;
        // Search the web works on any text selection. With several actions configured the icon
        // opens a menu, so each search engine or AI tool is one click away.
        let search = crate::icons::button(ui, "search", cell, false, tl!("Search the web"));
        if bar.web_tools.len() <= 1 {
            if search.clicked() && !bar.web_tools.is_empty() {
                chosen = Some(SelectionAction::Search(0));
            }
        } else {
            egui::Popup::menu(&search).show(|ui| {
                for (index, tool) in bar.web_tools.iter().enumerate() {
                    if ui.button(&tool.label).clicked() {
                        chosen = Some(SelectionAction::Search(index));
                        ui.close();
                    }
                }
            });
        }
        // The rest need a document that accepts comments or a copyable selection; the flag gates
        // both the visual state and the click.
        if bar.can_comment {
            if crate::icons::button(ui, "message-square-plus", cell, false, tl!("Add comment")).clicked() {
                chosen = Some(SelectionAction::Note);
            }
            for (icon, label, action) in [
                ("highlighter", tl!("Highlight"), SelectionAction::Markup(comments::CommentTool::Highlight)),
                ("underline", tl!("Underline"), SelectionAction::Markup(comments::CommentTool::Underline)),
                ("strikethrough", tl!("Strikethrough"), SelectionAction::Markup(comments::CommentTool::StrikeOut)),
                ("replace", tl!("Replace text"), SelectionAction::Replace),
            ] {
                if crate::icons::button(ui, icon, cell, false, label).clicked() {
                    chosen = Some(action);
                }
            }
        } else {
            // Draw the icons greyed out so the bar keeps its shape and the controls stay
            // discoverable, without acting.
            ui.add_enabled_ui(false, |ui| {
                crate::icons::button(ui, "message-square-plus", cell, false, tl!("Add comment"));
                for (icon, label) in [
                    ("highlighter", tl!("Highlight")),
                    ("underline", tl!("Underline")),
                    ("strikethrough", tl!("Strikethrough")),
                    ("replace", tl!("Replace text")),
                ] {
                    crate::icons::button(ui, icon, cell, false, label);
                }
            });
        }
        if bar.can_copy {
            if crate::icons::button(ui, "copy", cell, false, tl!("Copy")).clicked() {
                chosen = Some(SelectionAction::Copy);
            }
        } else {
            ui.add_enabled_ui(false, |ui| {
                crate::icons::button(ui, "copy", cell, false, tl!("Copy"));
            });
        }
    });
    chosen
}

/// The settings and permissions the selected-text bar reads.
pub(super) struct SelectionBar<'a> {
    pub prefs: &'a comments::CommentPrefs,
    pub web_tools: &'a [crate::webtools::WebTool],
    /// The document accepts comments (the markup tools are shown enabled).
    pub can_comment: bool,
    /// The selection can be copied.
    pub can_copy: bool,
}

/// Actions on an existing text selection, next to the selected words.
pub(super) fn selection_actions(ui: &mut egui::Ui, viewport: Rect, view: &mut DocView, info: &DocInfo, bar: &SelectionBar<'_>) -> bool {
    if ui.input(|input| input.pointer.is_decidedly_dragging()) {
        return false;
    }
    let Some(selection) = view.selection else {
        view.actions_dismissed = None;
        return false;
    };
    if view.actions_dismissed == Some(selection) {
        return false;
    }
    let Some(text) = view.texts.get(&selection.page) else { return false };
    let Some(xf) = view.page_xform(selection.page) else { return false };
    let Some(last) = selection.rects(text).last().copied() else { return false };
    let selected = selection.text(text);
    if selected.is_empty() {
        return false;
    }
    let rect = xf.view_rect(last);
    if !viewport.intersects(rect) {
        return false;
    }
    let x = rect.left().clamp(viewport.left() + 8.0, (viewport.right() - 520.0).max(viewport.left() + 8.0));
    let y = if rect.bottom() + 48.0 < viewport.bottom() { rect.bottom() + 6.0 } else { (rect.top() - 44.0).max(viewport.top() + 6.0) };
    let mut chosen = None;
    egui::Area::new(egui::Id::unique(("text-selection-actions", view.id.0))).order(egui::Order::Foreground).fixed_pos(pos2(x, y)).show(
        ui.ctx(),
        |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                chosen = actions_row(ui, bar);
            });
        },
    );
    let Some(action) = chosen else { return false };
    if let SelectionAction::Search(index) = action {
        // Open the selected words with that web action in the system browser, then close the
        // bar but keep the words selected, like Copy.
        if let Some(url) = bar.web_tools.get(index).and_then(|tool| tool.url_for(&selected)) {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
        }
        view.actions_dismissed = Some(selection);
        return false;
    }
    if let SelectionAction::Copy = action {
        // Copying closes the bar but keeps the words selected, for ⌘C or a second action.
        view.actions_dismissed = Some(selection);
        return true;
    }
    let Some((page, quads)) = view.selection_quads(info) else { return false };
    let last = quads.last().copied().unwrap_or_default();
    let at = [last[2].max(last[6]), last[1].max(last[3])];
    view.clear_selection();
    match action {
        SelectionAction::Note | SelectionAction::Replace => {
            if matches!(action, SelectionAction::Replace) {
                view.comments.replace = Some(quads);
            }
            view.comments.composer = Some(comments::Composer {
                page,
                at,
                kind: if matches!(action, SelectionAction::Replace) { comments::ComposerKind::Replace } else { comments::ComposerKind::Note },
                text: String::new(),
                focus: true,
            });
        }
        SelectionAction::Markup(tool) => {
            // Every markup button carries a markup tool; anything else adds nothing.
            let Some(kind) = tool.markup() else { return false };
            view.pending_edit = Some(Edit::AddAnnotation(pdfcraft_engine::NewAnnotation {
                page,
                shape: pdfcraft_engine::Shape::TextMarkup { kind, quads },
                style: bar.prefs.style(tool),
                contents: String::new(),
                author: bar.prefs.author.clone(),
            }));
        }
        // Copy is dispatched through the app after the view borrow ends; Search is
        // handled above, before the selection is cleared.
        SelectionAction::Copy | SelectionAction::Search(_) => {}
    }
    false
}

fn opacity_slider(ui: &mut egui::Ui, id: egui::Id) -> Option<f64> {
    ui.label(egui::RichText::new(tl!("Opacity")).strong());
    let mut pct = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(80.0);
    let changed = ui.add(egui::Slider::new(&mut pct, 10.0..=100.0).step_by(5.0).suffix(" %").trailing_fill(true)).changed();
    if changed {
        ui.data_mut(|d| d.insert_temp(id, pct));
        Some(f64::from(pct / 100.0))
    } else {
        None
    }
}

fn thickness_slider(ui: &mut egui::Ui, id: egui::Id) -> Option<f64> {
    ui.label(egui::RichText::new(tl!("Thickness")).strong());
    let mut w = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(2.0);
    let changed = ui.add(egui::Slider::new(&mut w, 0.5..=24.0).step_by(0.5).suffix(" pt").trailing_fill(true)).changed();
    if changed {
        ui.data_mut(|d| d.insert_temp(id, w));
        Some(f64::from(w))
    } else {
        None
    }
}

/// A compact colour panel for the selected highlight, underline or strikeout. Selecting one of
/// those marks (a click on the text) opens it, so its colour can be changed without the full
/// Comment Properties dialog. Returns the edit to apply, if a swatch was picked.
pub(super) fn markup_colors(ui: &mut egui::Ui, viewport: Rect, view: &DocView, info: &DocInfo) -> Option<Edit> {
    let (page, index) = view.comments.selected?;
    let a = info.annotations.iter().find(|a| a.page == page && a.index == index && !a.is_reply())?;
    let has_stroke = matches!(
        a.subtype.as_str(),
        "Ink" | "Line" | "Square" | "Circle" | "Polygon" | "PolyLine" | "Highlight" | "Underline" | "StrikeOut" | "Squiggly"
    );
    if a.locked || !has_stroke {
        return None;
    }
    let xf = view.page_xform(page)?;
    let rect = xf.user_rect(info, page, a.rect);
    if !viewport.intersects(rect) {
        return None;
    }
    let width = 232.0;
    let x = rect.left().clamp(viewport.left() + 8.0, (viewport.right() - width).max(viewport.left() + 8.0));
    let y = if rect.bottom() + 210.0 < viewport.bottom() { rect.bottom() + 6.0 } else { (rect.top() - 210.0).max(viewport.top() + 6.0) };
    let mut picked = None;
    let area_id = egui::Id::unique(("markup-colours", view.id.0, page, index));
    egui::Area::new(area_id).order(egui::Order::Foreground).fixed_pos(pos2(x, y)).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_max_width(width - 16.0);
            ui.label(egui::RichText::new(tl!("Colour")).strong());
            ui.add_space(4.0);
            if let Some(colour) = comments::swatch_grid(ui, a.color.map(|c| c.map(f64::from))) {
                picked = Some(Edit::StyleAnnotation { page, index, color: Some(colour), opacity: None, width: None, endings: None });
            }
            ui.add_space(6.0);
            if let Some(opacity) = opacity_slider(ui, area_id.with("opacity")) {
                picked = Some(Edit::StyleAnnotation { page, index, color: None, opacity: Some(opacity), width: None, endings: None });
            }
            if has_stroke {
                ui.add_space(6.0);
                if let Some(width) = thickness_slider(ui, area_id.with("thickness")) {
                    picked = Some(Edit::StyleAnnotation { page, index, color: None, opacity: None, width: Some(width), endings: None });
                }
            }
        });
    });
    picked
}
