//! Pinned tools on Home: pin tools, unpin tools, persistence across sessions,
//! selection modal, and the sidebar pin toggle.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{Dialog, LeftPanel, PdfCraftApp};

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.run_inline = true;
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

#[test]
fn state_operations_and_validation() {
    let mut app = PdfCraftApp::new();
    app.pinned_tools.clear();
    assert!(!app.is_tool_pinned("redact"));

    // Pinning a valid tool
    assert!(app.pin_tool("redact"));
    assert!(app.is_tool_pinned("redact"));
    assert_eq!(app.pinned_tools, vec!["redact"]);

    // Pinning already pinned tool returns false (no duplicates)
    assert!(!app.pin_tool("redact"));
    assert_eq!(app.pinned_tools, vec!["redact"]);

    // Pinning an invalid tool returns false
    assert!(!app.pin_tool("invalid_tool_unknown_id"));
    assert_eq!(app.pinned_tools, vec!["redact"]);

    // Pinning a second valid tool
    assert!(app.pin_tool("form"));
    assert_eq!(app.pinned_tools, vec!["redact", "form"]);

    // Unpinning
    app.unpin_tool("redact");
    assert!(!app.is_tool_pinned("redact"));
    assert!(app.is_tool_pinned("form"));
    assert_eq!(app.pinned_tools, vec!["form"]);

    // Unpinning non-existent tool does nothing
    app.unpin_tool("nonexistent");
    assert_eq!(app.pinned_tools, vec!["form"]);

    // Toggling
    assert!(!app.toggle_pin_tool("form")); // unpins
    assert!(!app.is_tool_pinned("form"));
    assert!(app.pinned_tools.is_empty());

    assert!(app.toggle_pin_tool("form")); // pins
    assert!(app.is_tool_pinned("form"));
    assert_eq!(app.pinned_tools, vec!["form"]);
}

#[test]
fn persistence_round_trip_and_sanitization() {
    let mut app = PdfCraftApp::new();
    app.pinned_tools.clear();
    app.pin_tool("protect");
    app.pin_tool("redact");

    let saved = app.persist();
    assert!(saved.contains("\"pinned_tools\""));

    let mut restored = PdfCraftApp::new();
    restored.restore(&saved);
    assert_eq!(restored.pinned_tools, vec!["protect", "redact"]);

    // Test sanitization: unknown tools are discarded and duplicates are removed, preserving order
    let malicious_or_corrupt = serde_json::json!({
        "pinned_tools": ["redact", "nonexistent_junk", "redact", "edit", "another_unknown", "protect"]
    })
    .to_string();
    restored.restore(&malicious_or_corrupt);
    assert_eq!(restored.pinned_tools, vec!["redact", "edit", "protect"]);
}

#[test]
fn home_pins_tools_only_when_there_are_some() {
    let mut h = harness(|app| {
        app.pinned_tools.clear();
    });
    // Empty Home stays minimal; pinning is still one click away.
    assert!(h.query_by_label("Pinned").is_none(), "no empty Pinned section");
    h.get_by_label("Pin a tool…");

    h.state_mut().pin_tool("redact");
    h.run_steps(3);
    h.get_by_label("Pinned");
    h.get_by_label("Redact a PDF");

    h.get_by_label("Unpin Redact a PDF").click();
    h.run_steps(3);
    assert!(h.query_by_label("Pinned").is_none(), "the section goes when empty");
    h.get_by_label("Pin a tool…");
}

#[test]
fn default_pinned_tools_present_on_fresh_install() {
    let app = PdfCraftApp::new();
    assert_eq!(app.pinned_tools, pdfcraft_ui_egui::default_pinned_tools());
    assert!(app.is_tool_pinned("edit"));
    assert!(app.is_tool_pinned("create"));
}

#[test]
fn many_pinned_tools_wrap_inside_home() {
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 1000.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        for g in pdfcraft_engine::catalog::TOOL_GROUPS {
            app.pin_tool(g.id);
        }
        app
    });
    h.run_steps(4);
    for g in pdfcraft_engine::catalog::TOOL_GROUPS {
        for node in h.query_all_by_label(pdfcraft_ui_egui::i18n::t(g.label)) {
            let r = node.rect();
            assert!(r.right() <= 900.0, "{} at {:?} runs off the Home column", g.id, r);
            assert!(r.left() >= 0.0, "{} at {:?}", g.id, r);
        }
    }
}

#[test]
fn clicking_pinned_tool_card_selects_it() {
    let mut h = harness(|app| {
        app.pin_tool("redact");
    });
    h.run_steps(2);

    // The Redact card is rendered and can be clicked to select the tool
    let nodes: Vec<_> = h.query_all_by_label("Redact a PDF").collect();
    assert!(!nodes.is_empty());
    // Click the card
    nodes.last().unwrap().click();
    h.run_steps(3);

    // Redact tool panel is now active
    assert_eq!(h.state().left, LeftPanel::Tool("redact"));
}

#[test]
fn pin_tool_dialog_workflow() {
    let mut h = harness(|app| {
        app.pinned_tools.clear();
        app.dialog = Some(Dialog::PinTool);
    });
    h.run_steps(3);

    // Modal is open
    h.get_by_label_contains("Pin tools to Home");
    h.get_by_label("Done");

    // Pin a tool from inside the dialog
    assert!(!h.state().is_tool_pinned("scan"));
    h.state_mut().toggle_pin_tool("scan");
    assert!(h.state().is_tool_pinned("scan"));
    h.run_steps(2);

    // Close dialog with "Done"
    h.get_by_label("Done").click();
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
}

const ONE_PAGE: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] >> endobj
trailer << /Root 1 0 R >>
%%EOF";

#[test]
fn drawer_pin_toggle_unpins_and_pins() {
    let mut h = harness(|app| {
        app.pinned_tools.clear();
        app.open_bytes("pinned.pdf", None, ONE_PAGE.to_vec()).expect("fixture opens");
        app.pin_tool("redact");
    });
    h.state_mut().drawer_open = true;
    h.run_steps(3);
    assert!(h.state().is_tool_pinned("redact"));

    // Since redact is pinned, its tile's pin is labelled "Unpin from Home".
    let nodes: Vec<_> = h.query_all_by_label("Unpin from Home").collect();
    for (i, n) in nodes.iter().enumerate() {
        println!("Node {i}: rect={:?}", n.rect());
    }
    nodes.first().unwrap().click();
    h.run_steps(3);

    assert!(!h.state().is_tool_pinned("redact"));
    // Left panel did not navigate away (still AllTools)
    assert_eq!(h.state().left, LeftPanel::AllTools);
}
