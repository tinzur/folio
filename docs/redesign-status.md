> **Superseded by ../HANDOFF.md (2026-11-07).** The test suite is now green, and focus mode
> and split view are implemented. This file is kept for the earlier design history.

# Reader redesign — status and handoff

Updated 2026-11-07. This note is for the next session; it records what changed, what is
decided, and what is still open.

## Direction

Turn PdfCraft into a distraction-free PDF reader:

- two full-width bands at the top: a document row and an annotation row (the annotation
  row can be one or two rows of tools);
- no right navigation rail, no mode tab bar, no permanent left tools panel;
- a tools drawer behind a hamburger, with groups that expand in place;
- Monochrome as a switch over Light/Dark (dark monochrome is the reading default);
- page/zoom/view/panels compact at the right of the tool row;
- tool options open on the **left**; comments/bookmarks/pages etc. stay on the right;
- one or two annotation rows, chosen by the reader, with editable contents;
- split view (two PDFs side by side) is Phase 3 and is **not started**.

## Done

- `ThemeKind::Mono` was replaced by a `PdfCraftApp::monochrome` switch that follows the
  current Light/Dark palette (`theme::Tokens::for_kind_mono`). Persisted as `monochrome`.
- Dark and light monochrome palettes; tool group hues collapse to the icon ink.
- Dark-monochrome Preferences contrast fixed: the preference cards were light on a dark
  dialog; active pills/buttons now use `Tokens::on_accent()` for readable ink.
- The window has a full-width document row (`chrome::tab_strip`) and a full-width annotation
  row (`hotbar::strip`, reserved by the `tool_row` top panel). Side panels start below both.
- Discord and the help/theme/settings icons were removed from the title bar's right side.
  Home, shortcuts, and theme now sit together on the left. Search, Properties, Save and Print
  are at the right of the document row; Settings is at the right of the tool row.
- The mode tab bar and the right navigation rail were removed. Workspaces live in the drawer.
  Page/zoom/page-display/history/panels/settings live at the right of the tool row.
- The tools drawer (`drawer.rs`) overlays the page below the rows, with search, a collapsed
  workspace section, and a catalog. Groups expand in place. Escape/outside click closes it.
- The annotation row is curated (`PdfCraftApp::strip_tools`, persisted as `strip_tools`),
  with Select/Hand fixed, a Customize button, and a toolbar editor in the drawer.
- Tool options dock left; the inspector (Comments, Bookmarks, Pages, fields, layers,
  attachments, signatures, tags) docks right. The Tools pane is only shown for a tool with
  its own options (organize, form, stamp, measure, redact, edit), or the organize grid.
- The old floating quick palette remains available as an option (`floating_toolbar`), off by
  default.
- Read mode keeps the tool row and closes the panes.
- Annotation rows are dynamic: **Auto (max 2)** is the default. `strip_layout` measures the
  tools, the tail and the compact reading controls against the window width; if they do not
  fit, it fills the first row and wraps the overflow to a second, never more. A wide desktop
  keeps one row; a narrow laptop window wraps. The drawer's toolbar editor also offers fixed
  **1 row** and **2 rows**, an icon-size slider (22–40), and a live “Fitting: N row(s)” readout.
  `strip_rows` / `strip_rows_auto` are persisted; `strip_rows_shown` is the last drawn count.
  `strip_height` uses the same measurement, so the reserved band matches in the same frame.
- The drawer is a tile grid again: group cards are two columns, and an open group drops its
  matching item tiles (three columns) under its row, pushing the rest of the catalog down.
- The reading controls are compact: 26 px controls, 30 px page field, 38 px zoom field, no
  gaps between the cluster buttons.
- Tool options dock back on the **left**; the inspector (Comments etc.) stays on the right.
- A pinned "Links & help" footer at the bottom of the drawer carries Discord, keyboard
  shortcuts, Preferences, About, and GitHub (the title bar no longer does).
- Switching a workspace in the drawer no longer closes it; only choosing a tool or an action
  does. (The old behavior made the drawer vanish when moving from Read to All tools.)
- New tests cover the drawer, toolbar editor (including the 1/2-row switch), monochrome
  round-trip, and Read mode.
- `cargo fmt`, `cargo clippy -p pdfcraft-ui-egui --all-targets -- -D warnings`, and the
  library tests (750) pass.

## Open

1. **Focus mode** (auto-hide rows while reading) and **split view** remain unbuilt.
2. **Legacy test migration** (see below).
3. Optional polish: give the drawer a single tabbed right-hand sheet instead of stacking tool
   options and Comments.

## Known test state

1361 of 1443 integration tests pass; 78 fail. Most failures are the old chrome tests that
still drive the removed rail, mode tabs, left All-tools panel, or fixed page coordinates.
One failure is this machine's missing U+FFFD glyph. The legacy suite needs migration to the
new chrome; do not treat the failures as new runtime bugs without checking the test.

## Files touched

- `crates/ui-egui/src/theme.rs`
- `crates/ui-egui/src/app.rs`, `app/frame.rs`, `app/prefs.rs`, `app/control.rs`
- `crates/ui-egui/src/chrome.rs` (document row; rail/mode bar removed)
- `crates/ui-egui/src/hotbar.rs` (annotation row, reading controls)
- `crates/ui-egui/src/drawer.rs` (tools drawer)
- `crates/ui-egui/src/canvas.rs` (drawer/row integration)
- `crates/ui-egui/src/panels/side_panels.rs` (left tool options, right inspector)
- `crates/ui-egui/src/panels/tools.rs`, `panels/all_tools_ui.rs`, `dialogs/appearance.rs`,
  `dialogs/preferences.rs`, `dialogs.rs`, `widgets.rs`
- `crates/engine/src/commands.rs` (`view.monochrome`)
- tests under `crates/ui-egui/tests/` and `crates/ui-egui/src/**/tests.rs`
