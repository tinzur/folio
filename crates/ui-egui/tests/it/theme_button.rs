//! The title bar's theme button: a click cycles System, Light and Dark (a fresh install starts
//! on System, #1248); a right-click opens the menu with the appearance settings.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(6);
    h
}

fn preference(h: &Harness<'static, PdfCraftApp>) -> String {
    format!("{:?}", h.state().theme_preference)
}

#[test]
fn the_tool_rows_settings_gear_opens_preferences() {
    let mut h = harness();
    // The row (and its settings gear) belongs to a document; on Home, Preferences is in the
    // application menu.
    h.state_mut()
        .open_bytes(
            "doc.pdf",
            None,
            b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF"
                .to_vec(),
        )
        .unwrap();
    h.run_steps(3);
    h.get_by_label("Preferences").click();
    h.run_steps(6);
    assert!(h.query_by_label("Appearance mode").is_some());
    assert!(h.query_by_label("Light theme").is_some());
    assert!(h.query_by_label("Dark theme").is_some());
    assert_eq!(preference(&h), "System");
}

#[test]
fn a_click_cycles_the_theme_and_a_right_click_opens_the_menu() {
    let mut h = harness();
    assert_eq!(preference(&h), "System", "a fresh install follows the desktop (#1248)");
    for (label, next) in
        [("Display theme: Use system setting", "Light"), ("Display theme: Light gray", "Dark"), ("Display theme: Dark gray", "System")]
    {
        h.get_by_label(label).click();
        h.run_steps(4);
        assert_eq!(preference(&h), next);
    }
    assert!(h.query_by_label("Appearance settings…").is_none(), "a plain click opens no menu");

    h.get_by_label("Display theme: Use system setting").click_secondary();
    h.run_steps(4);
    h.get_by_label("Appearance settings…").click();
    h.run_steps(4);
    h.run_steps(10);
    assert!(h.query_by_label("Light theme").is_some(), "Preferences opens on Appearance");
}
