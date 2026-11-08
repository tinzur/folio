//! The developer and source links: no ArtCraft promotion, no update prompt, honest attribution.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::links;
use pdfcraft_ui_egui::{Dialog, PdfCraftApp};

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

#[test]
fn home_has_the_developer_and_source_links_only() {
    let mut h = harness(|_| {});
    for gone in ["Join the ArtCraft community", "Join our Discord", "ArtCraft website", "PdfCraft web page"] {
        assert!(h.query_by_label(gone).is_none(), "{gone} must not be on Home");
    }
    assert_eq!(links::TELEGRAM, "https://t.me/Tinzur");
    assert!(links::GITHUB.contains("pdfcraft"));

    h.get_by_label("Telegram — @TINZUR").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::TELEGRAM));

    h.get_by_label("PdfCraft source").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::GITHUB));

    // The redesign is stated, not hidden behind a brand.
    h.get_by_label_contains("is an independent, distraction-free reading shell by TINZUR");
    h.get_by_label_contains("built on the open-source PdfCraft project");
}

#[test]
fn home_sections_are_customizable() {
    let mut h = harness(|_| {});
    assert!(h.query_by_label("Recent").is_some());
    h.get_by_label("Customize home").click();
    h.run_steps(2);
    h.get_by_label("Recent files").click();
    h.run_steps(3);
    assert!(h.query_by_label("Recent").is_none(), "Recent turned off");
    assert!(h.query_by_label("Recent").is_none());
    // Reopen if the menu closed, then turn it back on.
    if h.query_by_label("Customize home").is_some() {
        h.get_by_label("Customize home").click();
        h.run_steps(2);
    }
    if h.query_by_label("Recent files").is_some() {
        h.get_by_label("Recent files").click();
        h.run_steps(3);
        assert!(h.query_by_label("Recent").is_some(), "Recent turned back on");
    }
}

#[test]
fn about_dialog_shows_the_new_name_and_both_links() {
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    let mut h = harness(move |app| {
        app.open_bytes("one.pdf", None, pdf.to_vec()).unwrap();
        app.dialog = Some(Dialog::About);
    });
    h.get_by_label_contains("a distraction-free PDF reader");
    assert_eq!(h.query_all_by_label("ArtCraft").count(), 0, "no ArtCraft mark");

    h.get_by_label("Telegram — @TINZUR").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::TELEGRAM));
}

#[test]
fn help_commands_open_each_link_and_updates_are_gone() {
    for l in links::LINKS {
        let mut h = harness(|_| {});
        assert!(h.state_mut().execute(l.command), "{}", l.command);
        assert_eq!(h.state().last_opened_url.as_deref(), Some(l.url));
    }
    assert!(pdfcraft_engine::commands::command("help.discord").is_none());
    assert!(pdfcraft_engine::commands::command("help.website").is_none());
    assert!(pdfcraft_engine::commands::command("help.app_page").is_none());
    assert!(pdfcraft_engine::commands::command("help.check_updates").is_none(), "the updater is not offered");
}

/// Runs frames until the node labelled `label` exists and has stopped moving, then returns it.
fn settled<'a>(h: &'a mut Harness<'static, PdfCraftApp>, label: &'a str) -> egui_kittest::Node<'a> {
    let mut last = None;
    for _ in 0..60 {
        h.run_steps(1);
        let now = h.query_by_label(label).map(|n| n.rect());
        if now.is_some() && now == last {
            return h.get_by_label(label);
        }
        last = now;
    }
    panic!("{label:?} never appeared or never stopped moving");
}

#[test]
fn about_dialog_has_contributors_and_models_tabs() {
    let mut h = harness(|app| app.dialog = Some(Dialog::About));
    settled(&mut h, "Contributors").click();
    settled(&mut h, "@echelon");
    settled(&mut h, "Table").click();
    settled(&mut h, "PRs");
    settled(&mut h, "Display name").click();
    settled(&mut h, "Brandon Thomas");
    settled(&mut h, "Models").click();
    h.run_steps(4);
    assert!(h.query_all_by_label("Anthropic").count() >= 1);
}
