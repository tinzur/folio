//! Side panels: the Tools pane, the rail and the open side panel.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;

const FIXTURE: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] >> endobj
trailer << /Root 1 0 R >>
%%EOF";

const WIDTH: f32 = 1400.0;
const NO_BOOKMARKS: &str = "This document has no bookmarks.";

fn harness_sized(size: egui::Vec2, setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(size).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, FIXTURE.to_vec()).unwrap();
        setup(&mut app);
        app
    });
    h.run_steps(6);
    h
}

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    harness_sized(egui::vec2(WIDTH, 900.0), setup)
}

fn side(h: &Harness<'static, PdfCraftApp>) -> egui::Rect {
    h.get_by_label(NO_BOOKMARKS).rect()
}
#[test]
fn a_dragged_width_is_remembered_across_restarts() {
    let mut h = harness(|app| app.set_option("panel", "bookmarks").unwrap());
    let before = h.state().side_panels.side_width;
    let side_left = side(&h).left();
    // The side panel's resize edge: the one right of the page, left of the panel's text.
    let edge = h
        .get_all_by_label("Resize panels")
        .map(|n| n.rect().left_center())
        .filter(|c| c.x < side_left && c.x > WIDTH / 2.0)
        .min_by(|a, b| (side_left - a.x).total_cmp(&(side_left - b.x)))
        .expect("the side panel has a resize edge")
        // 2 pt inside the panel: the edge lies right of the border, clear of the page's scrollbar.
        + egui::vec2(2.0, 0.0);
    h.hover_at(edge);
    h.run_steps(1);
    h.drag_at(edge);
    h.run_steps(1);
    for i in 1..=4 {
        h.hover_at(edge - egui::vec2(10.0 * i as f32, 0.0));
        h.run_steps(1);
    }
    h.drop_at(edge - egui::vec2(40.0, 0.0));
    h.run_steps(3);
    let after = h.state().side_panels.side_width;
    // The pointer was pressed 2 pt inside the panel, and the edge follows the pointer.
    assert!((after - (before + 40.0 - 2.0)).abs() <= 1.0, "{before} -> {after}");
    let saved = h.state().persist();
    let mut restored = PdfCraftApp::new();
    restored.restore(&saved);
    assert_eq!(restored.side_panels.side_width, after);
}

#[test]
fn f4_toggles_the_side_panel_and_shift_f4_the_tools_panel() {
    let mut h = harness(|app| app.set_option("panel", "bookmarks").unwrap());
    h.get_by_label(NO_BOOKMARKS);
    h.key_press(egui::Key::F4);
    h.run_steps(3);
    assert_eq!(h.state().right, None);
    assert!(h.query_by_label(NO_BOOKMARKS).is_none());
    // F4 again reopens the panel that was open, not the default.
    h.key_press(egui::Key::F4);
    h.run_steps(3);
    h.get_by_label(NO_BOOKMARKS);

    // Shift+F4 toggles the left tool-options pane when a tool is selected.
    h.state_mut().set_option("tool", "edit").unwrap();
    h.run_steps(3);
    assert!(h.state().left_open);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F4);
    h.run_steps(3);
    assert!(!h.state().left_open);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F4);
    h.run_steps(3);
    assert!(h.state().left_open);
    // F4 leaves the tools pane alone.
    h.key_press(egui::Key::F4);
    h.run_steps(3);
    assert!(h.state().left_open);
}
