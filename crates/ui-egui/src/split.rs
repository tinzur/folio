//! Browser-style split view: two documents side by side, one divider, one focused pane.
//! The focused pane is the one the tool row, panels and keyboard act on; clicking a pane focuses
//! it. The active tab is the focus indicator: no second title bar or border is drawn over the
//! pages. Only two panes exist; a third document leaves split view and shows single.

use egui::{CornerRadius, Rect, Sense, Stroke, pos2, vec2};

use crate::theme::Tokens;
use crate::{PdfCraftApp, QuickTool, SplitSide, icons};

const DIVIDER: f32 = 6.0;

/// Keep `app.active` and the split's focused pane in step. Called at the start of the frame.
pub(crate) fn restore(app: &mut PdfCraftApp) {
    if app.compare.is_some() || app.combine_showing() {
        return;
    }
    let Some(saved) = app.split_saved else { return };
    if app.split.is_some() {
        app.split_saved = None;
        return;
    }
    if !app.views.iter().any(|v| v.id == saved.left) || !app.views.iter().any(|v| v.id == saved.right) {
        app.split_saved = None;
        return;
    }
    let Some((_, active)) = app.active_ids() else { return };
    let mut split = saved;
    if active == split.left {
        split.focus = SplitSide::Left;
    } else if active == split.right {
        split.focus = SplitSide::Right;
    } else {
        return;
    }
    app.split = Some(split);
    app.split_saved = None;
}

pub(crate) fn sync(app: &mut PdfCraftApp) {
    let Some(mut split) = app.split else { return };
    if !app.views.iter().any(|v| v.id == split.left) || !app.views.iter().any(|v| v.id == split.right) {
        app.split = None;
        return;
    }
    let Some((_, active)) = app.active_ids() else {
        app.split_saved = Some(split);
        app.split = None;
        return;
    };
    if active == split.left {
        split.focus = SplitSide::Left;
    } else if active == split.right {
        split.focus = SplitSide::Right;
    } else {
        // A third document chosen: it shows alone, but the group is remembered so returning to
        // either of its documents brings the split back.
        app.split_saved = Some(split);
        app.split = None;
        return;
    }
    let focus_id = match split.focus {
        SplitSide::Left => split.left,
        SplitSide::Right => split.right,
    };
    app.split = Some(split);
    app.active = app.views.iter().position(|v| v.id == focus_id);
}

/// Draw the two pages and the draggable divider. Returns false when the split was left (a tab
/// click chose a third document); the caller then draws the ordinary single view.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) -> bool {
    let Some(mut split) = app.split else { return false };
    let Some((_, active_id)) = app.active_ids() else {
        app.split_saved = Some(split);
        app.split = None;
        return false;
    };
    // A tab chosen in the document row during this frame: if it is a split document focus that
    // side, otherwise leave split view before its old focused pane can overwrite the choice.
    if active_id == split.left {
        split.focus = SplitSide::Left;
    } else if active_id == split.right {
        split.focus = SplitSide::Right;
    } else {
        app.split_saved = Some(split);
        app.split = None;
        return false;
    }
    app.split = Some(split);

    let full = ui.max_rect();
    if full.width() < 160.0 {
        // Too narrow for two readable pages: fall back to the focused single view, keeping the
        // group remembered for when the window grows again.
        app.split_saved = Some(split);
        app.split = None;
        return false;
    }
    let t = Tokens::get(ui.ctx());
    let ratio = split.ratio.clamp(0.2, 0.8);
    let left_width = (full.width() - DIVIDER) * ratio;
    let left = Rect::from_min_size(full.min, vec2(left_width, full.height()));
    let divider = Rect::from_min_size(pos2(left.right(), full.top()), vec2(DIVIDER, full.height()));
    let right = Rect::from_min_size(pos2(divider.right(), full.top()), vec2(full.right() - divider.right(), full.height()));

    // A press anywhere in a pane focuses it before the click is handled. With scroll lock on,
    // a wheel/trackpad scroll also makes the pane under the pointer the leader.
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let scrolling = split.scroll_lock && ui.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO);
    // Drawer, palette and dialogs sit above the panes. A click on one of them must not also
    // focus the pane it happens to overlap (the left pane lies under the tools drawer).
    let overlay = app.drawer_open || app.palette_open || app.modal_open();
    if !overlay
        && (ui.input(|i| i.pointer.any_pressed()) || scrolling)
        && let Some(p) = pointer
    {
        if left.contains(p) {
            split.focus = SplitSide::Left;
        } else if right.contains(p) {
            split.focus = SplitSide::Right;
        }
    }
    let focus_id = match split.focus {
        SplitSide::Left => split.left,
        SplitSide::Right => split.right,
    };
    if let Some(index) = app.views.iter().position(|v| v.id == focus_id) {
        app.switch_active_view(index);
    }
    app.split = Some(split);

    let drag = ui.interact(divider, ui.scope_id().with("split-divider"), Sense::drag());
    if drag.hovered() || drag.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if drag.dragged() {
        split.ratio = (split.ratio + drag.drag_delta().x / full.width()).clamp(0.2, 0.8);
        app.split = Some(split);
        app.split_ratio = split.ratio;
    }

    pane(app, ui, left, SplitSide::Left, split.left, split.focus == SplitSide::Left);
    pane(app, ui, right, SplitSide::Right, split.right, split.focus == SplitSide::Right);

    if split.scroll_lock {
        // One pane leads; the other follows its offset next frame (the same mechanism Compare
        // uses). The leader is the focused pane, or the pane under the pointer while scrolling.
        let focus = match split.focus {
            SplitSide::Left => app.views.iter().position(|v| v.id == split.left),
            SplitSide::Right => app.views.iter().position(|v| v.id == split.right),
        };
        let other = match split.focus {
            SplitSide::Left => app.views.iter().position(|v| v.id == split.right),
            SplitSide::Right => app.views.iter().position(|v| v.id == split.left),
        };
        if let (Some(focus), Some(other)) = (focus, other) {
            let offset = app.views[focus].last_scroll;
            app.views[other].sync_scroll = Some(offset);
        }
    }

    ui.painter().rect_filled(divider, CornerRadius::ZERO, t.pasteboard);
    ui.painter().vline(divider.center().x, divider.y_range(), Stroke::new(1.0, if drag.hovered() { t.accent } else { t.divider }));
    if drag.hovered() {
        ui.painter().rect_filled(divider.shrink2(vec2(2.0, 0.0)), CornerRadius::same(2), t.hover);
    }

    // Dedicated Close-Pane control directly on the divider
    let close_size = 20.0;
    let close_center = pos2(divider.center().x, divider.top() + 18.0);
    let close_rect = Rect::from_center_size(close_center, vec2(close_size, close_size));
    let close_resp = ui.interact(close_rect, ui.scope_id().with("close-split-divider"), Sense::click());
    if close_resp.hovered() {
        ui.painter().circle_filled(close_center, 10.0, t.card);
        ui.painter().circle_stroke(close_center, 10.0, Stroke::new(1.0, t.border));
        icons::paint(ui, close_rect.shrink(5.0), "x", 10.0, t.accent);
    }
    if close_resp.on_hover_text(tl!("Close Split View")).clicked() {
        app.split = None;
        app.split_saved = None;
        return false;
    }

    // The tab strip and the tool row read `active`: leave it on the focused pane.
    app.split = Some(split);
    app.active = app.views.iter().position(|v| v.id == focus_id);
    true
}

fn pane(app: &mut PdfCraftApp, ui: &mut egui::Ui, rect: Rect, side: SplitSide, id: pdfcraft_engine::DocId, focused: bool) {
    let Some(index) = app.views.iter().position(|v| v.id == id) else { return };
    let resp = ui.interact(rect, ui.scope_id().with(("split-focus", id.0)), Sense::click());
    if resp.clicked() {
        if let Some(mut split) = app.split {
            split.focus = side;
            app.split = Some(split);
        }
        app.active = Some(index);
    }
    let t = Tokens::get(ui.ctx());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("split-pane", id.0)));
    child.set_clip_rect(rect);
    child.painter().rect_filled(rect, CornerRadius::ZERO, t.pasteboard);
    let saved_tool = app.quick_tool;
    if !focused {
        // A click focuses (above) before this pane is drawn; Select keeps a live annotation tool
        // from marking the other document by accident.
        app.quick_tool = QuickTool::Select;
    }
    app.active = Some(index);
    crate::canvas::document_area(app, index, &mut child);
    app.quick_tool = saved_tool;
}
