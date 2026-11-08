# Folio / PdfCraft reader — current handoff

**Read this first.** This supersedes the older redesign notes. Written 2026-11-08 after the
split-view consistency and horizontal-lock work.

## 1. Start here

Project (use this exact path; it contains spaces):

```bash
cd "/Users/shyam/Downloads/New Project Of Mine/pdfcraft-main"
```

This is a **git repository** (initialised 2026-11-08, branch `main`). `git diff`, branches and a
clean checkout all work; commit before starting a change and after it passes the tests.
`~/Downloads/Pdfcraft` is an empty folder and is not the project.

Version control basics:

```bash
git status                  # what changed
git diff                    # what changed, in detail
git add -A && git commit -m "…"   # a checkpoint
git checkout -- <file>      # throw away one file's changes
git revert <sha>            # undo a commit, keeping history
git tag -a v0.18.2 -m "0.18.2"
```

There is no remote yet. `target/`, `dist/`, `assets/models/` and `.perch/` are ignored, so a
commit is about 43 MB of source, not the 16 GB build tree. Work on short-lived branches
(`feat/…`, `fix/…`) off `main`; `main` always builds and always passes the tests below.

The app is a Rust/egui fork/rebrand of the open-source **PdfCraft** engine. The current display
name is the **working name “Folio”**, not a final naming decision. The user is TINZUR
(https://t.me/Tinzur); the QR code in the app is their Telegram profile, not an app logo.

Build and run without making a DMG:

```bash
cargo build -p pdfcraft
target/debug/pdfcraft "/path/to/file.pdf"
# or
cargo run -p pdfcraft
```

The user explicitly does **not** want a DMG built after every change. Build one only at the end,
when they ask for the final DMG. The last DMG predates the latest horizontal-lock work:

```text
dist/release/folio-0.18.1-macos-aarch64.dmg
```

It is 37 MB, arm64-only, ad-hoc signed, includes the OCR models, and is **not the latest code**.
For normal testing use the debug app above.

Before judging a change, make sure an old `target/debug/pdfcraft` process/file is not what is
being tested; rebuild and relaunch.

## 2. Current status

Last verified run:

- **1426 integration tests pass, 0 fail, 1 ignored**
- **747 library tests pass, 1 ignored**
- `cargo fmt --check` clean
- `cargo clippy -p pdfcraft-ui-egui --all-targets -- -D warnings` clean
- Debug app builds; live control-channel testing works.

Commands:

```bash
cargo test -p pdfcraft-ui-egui --test it
cargo test -p pdfcraft-ui-egui --lib
cargo fmt
cargo clippy -p pdfcraft-ui-egui --all-targets -- -D warnings
```

The one ignored integration test is the real, deferred **Add-text paragraph drag** regression:

```text
editing::add_text_finishes_a_paragraph_drag_that_leaves_its_box
```

The user said to leave it for later. Do not delete it; fix the content-tool behavior when time
allows.

The library has one pre-existing ignored test; it is not part of the UI work.

## 3. Product intent and user preferences

The goal is a **distraction-free PDF reader**, not an Acrobat-style workbench.

- Monochrome black-and-white chrome over Light or Dark. The page keeps its real content colours.
- Reading space is the priority; chrome is two full-width rows:
  1. document/tab row;
  2. annotation/tool row (auto 1–2 rows, max 2; compact page/zoom at the right).
- Tools live under a hamburger drawer; groups expand in place; boxed tiles, not line rows.
- Tool options panel on the left; comments/inspector on the right.
- Workspaces (Read/Edit/Convert/E-Sign) are available in the drawer. Choosing a workspace other
  than All tools opens its panel and closes the drawer. All tools keeps the drawer.
- Focus mode hides the annotation row while reading; it returns at the top edge or via the eye
  button in the document row.
- Split view is exactly two PDFs, browser-style, with independent per-pane tools and settings.
- No ArtCraft branding, Discord/website promotional links, or update prompt.
- Honest attribution: built on the open-source PdfCraft project, redesigned by TINZUR for focused
  reading. The original engine is credited; the new shell is described as this fork’s work.
- Local/private: no account or telemetry.

## 4. What is implemented

### 4.1 Chrome and home

- Two-row shell in `crates/ui-egui/src/app.rs`, document row in `chrome.rs`, annotation row in
  `hotbar.rs`; auto/fixed 1–2 rows, compact reading controls.
- Full-width tools drawer in `drawer.rs`, with working workspace chips and boxed catalog tiles.
- Focus mode (`view.focus_mode`, ⇧⌘F, eye button) and persisted preference.
- New minimal Home in `home.rs`:
  - Open PDF, Actions, Pinned, Recent, Developer/credits.
  - `Customize home` toggles every section and changes the recent limit; persisted as
    `home_show_pinned`, `home_show_recent`, `home_show_quick`, `home_show_developer`.
  - Pinned tools use fixed-width chips in a wrapping row; this fixed the overflow to the right.
  - Pinned folders are listed below the tool chips; Add tools/folders is in Actions.
  - Developer card contains TINZUR’s Telegram link and QR.

### 4.2 Split view

`crates/ui-egui/src/split.rs`, `SplitView` in `lib.rs`:

- Exactly two panes; draggable divider; both panes render.
- No duplicate pane title bars and no focus border. The **active tab** is the focus indicator.
- The split group accent is drawn under both tabs, but only the focused tab gets the accent; the
  other gets a muted rule.
- Per-pane tool state (`DocView::tool` + `tool_owner`); Edit/E-Sign/workspaces follow focus.
- Split picker is a two-column grid; same-PDF copy is supported; one PDF asks to open another.
- A third PDF leaves split but remembers the pair (`split_saved`); returning to either document
  restores the pair.
- Split scroll lock: a lock button in the document row synchronizes the two panes’ scrolling.
- Horizontal position lock: per-view `DocView::hscroll_lock`, button between page and zoom;
  prevents auto-centering on page jumps/fits when page widths differ. Normal and split modes.
- Combine and Compare remember the split pair and restore it when closed.
- Compare has **Back to reading** in the panel and in the comparison view; closing clears the
  selected old file and restores split.
- `all_tools_full` is cleared when entering Combine/Compare so it cannot hide the restored pair.

### 4.3 Branding, links, updates, OCR

- Display name: **Folio** in `crate::APP_NAME`; window title, Home, About, native menus, README,
  packaging and DMG use it. The internal crate/binary remains `pdfcraft`.
- New page-and-fold app icon generated into `assets/app-icon/`; `.icns`/`.ico`/hicolor updated.
- Telegram: `https://t.me/Tinzur`, `links::TELEGRAM`; QR at `assets/brand/tinzur-qr.png`.
- Source credit: `https://github.com/storytold/pdfcraft`.
- Removed ArtCraft website/Discord/app-page links, ArtCraft brand assets (`docs/brand/`),
  command entries and home/about cards. Do not add them back.
- Startup update checks are off; Help no longer has Check for updates; `help.check_updates` was
  removed. The update module remains but is unreachable. The stale
  `~/Library/Application Support/PdfCraft/self-update.txt` was removed.
- OCR is included in the DMG:
  - Models are the author-published OCrs ONNX files (Robert Knight), unmodified, CC-BY-SA-4.0.
  - Pins in `ATTRIBUTION.toml`: detection `a917…`, recognition `86c…`, licence text `28a9…`.
  - All 12 OCR integration tests pass.
  - `Folio.app/Contents/Resources/models` contains the two models, licences and attribution.
- Packaging macOS script now builds `Folio.app`/DMG names; ad-hoc signing only unless
  `MACOS_SIGN_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` are supplied. It is not
  notarized.

## 5. Important internals

- `crates/ui-egui/src/app.rs`:
  - frame order: split restore/sync → per-doc tool load → tab strip → focus-mode decision →
    side panels → central (split/all-tools/home/document) → tool row → drawer → dialogs.
  - `switch_active_view(index)` saves/loads the per-document tool and makes Edit/E-Sign follow
    the focused pane’s tool. It is used by split panes.
  - The end-of-frame tool save only runs when `tool_owner == active document`; this prevents a
    tab click from copying the old tab’s tool into the new one.
- `crates/ui-egui/src/split.rs`:
  - `sync`/`restore` keep `SplitView.focus` and `app.active` together.
  - Pane pre-focus is skipped while a **drawer, palette or dialog** is open. This was the common
    cause of tools applying to the first/left PDF: the drawer overlays the left pane, and its
    clicks were also focusing that pane. Do not remove this guard.
  - `scroll_lock` synchronizes both panes; `DocView.hscroll_lock` is separate and per-view.
- `crates/ui-egui/src/canvas.rs`:
  - `DocView.hscroll_lock` makes the `goto` branch preserve `last_scroll.x` instead of centering
    the new (possibly differently sized) page.
- `crates/ui-egui/src/chrome.rs`:
  - split lock button, focus eye, active tab accent. Keep the in-window menu bar on platforms
    without a native menu.
- `crates/ui-egui/src/home.rs`:
  - fixed-width pinned chips; don’t revert to a Frame around a child horizontal (it claimed the
    full remaining width and overflowed).
- Tests:
  - `crates/ui-egui/tests/it/ui.rs`: split focus, fit per pane, tool modes, scroll locks,
    Combine/Compare return, focus mode, tabs.
  - `pinned_tools.rs`: narrow Home pin reflow.
  - `links.rs`: Telegram/source/About, no ArtCraft.
  - `ocr.rs`: all 12 OCR flows.
  - `panel_snapshots.rs`: snapshot `panel_compare.txt` was updated for the new Compare header.
  - Snapshot update: `UPDATE_SNAPSHOTS=1 cargo test -p pdfcraft-ui-egui --test it panel_snapshots::every_panel_matches_its_snapshot`.
- `links.rs`, `menus.rs`, `drawer.rs`, `dialogs.rs`, `README.md`, `NOTICE`, packaging:
  branded/cleaned. `docs/brand` was deleted; `assets/brand/tinzur-qr.png` remains.

## 6. Live testing without a DMG

Preferred when checking UI fixes:

```bash
cd "/Users/shyam/Downloads/New Project Of Mine/pdfcraft-main"
cargo build -p pdfcraft
rm -f /tmp/folio-control
target/debug/pdfcraft --control /tmp/folio-control "/path/to/file.pdf"
```

Then use the CLI in another terminal:

```bash
# Build the CLI once:
cargo build -p pdfcraft-cli
CLI="target/debug/pdfcraft-cli"
"$CLI" ui --control /tmp/folio-control ui.state
"$CLI" ui --control /tmp/folio-control ui.open path="/path/to/file.pdf"
"$CLI" ui --control /tmp/folio-control ui.inspect query=Split
"$CLI" ui --control /tmp/folio-control ui.click label="Lock horizontal position"
"$CLI" ui --control /tmp/folio-control ui.screenshot --out /tmp/folio.png
"$CLI" ui --control /tmp/folio-control ui.quit
```

The control channel is the quickest way to reproduce split/focus/tool behavior without screenshots
or a packaged app. The app’s screenshot command returns a real render even when macOS screen
capture is unavailable.

## 7. What to do next

1. **Final name/branding.** The user has not chosen the final name. Current working name is
   **Folio**. Suggested alternatives: Quarto, Margin, Still, Plain, Leaf, Mono. Changing it means
   `APP_NAME`, README, packaging strings and a rebuild. Do not treat Folio as final.
2. **Fix the Add-text paragraph drag** (the one ignored test). It is a content-tool regression,
   not a chrome leftover.
3. **Manually verify the horizontal lock on a truly mixed-size PDF** (A4 + wide page, normal and
   split). The code path and per-pane toggle test are green, but a real mixed-width visual pass is
   still useful.
4. **Remaining split polish** (not yet implemented): per-pane PDF picker without adding a title
   bar, a dedicated close-pane control (the split button toggles off), and persisting the split
   group across application restarts (the pair is remembered in-session; ratio persists).
5. **Focus polish** (optional): hide the document row/side panels in focus mode and add an idle
   delay/affordance.
6. **Final DMG** only when requested:
   - Current build is arm64-only; universal needs `x86_64-apple-darwin` target installed.
   - Ad-hoc signing is fine locally; distribution needs Developer ID + notarization.
   - OCR models are included; don’t drop them.
   - Run `MACOS_SIGN_IDENTITY="-" packaging/macos/package.sh --arch aarch64` (no
     `PDFCRAFT_SKIP_MODELS`; the models verify now).
   - Test by mounting the DMG and launching via control channel.
7. Re-run full tests, clippy, fmt and a final visual pass.

## 8. Do not break

- The overlay guard in `split::show`: tool/drawer clicks must not focus the pane underneath.
- Per-document `tool_owner` save rule: a tab switch must not copy one PDF’s tool into another.
- `split_saved` / Combine / Compare return behavior.
- Active tab accent must follow the focused pane; inactive group tab is muted.
- Pinned chips must wrap; no full-width frames inside the pinned row.
- OCR models and their CC-BY-SA attribution/licence files must remain in the app bundle.
- No ArtCraft links/assets or update prompt.
- The internal crate remains `pdfcraft`; changing the display name is a UI/packaging change, not
  a workspace-package rename, unless the user explicitly asks for that larger migration.
- Do not run `cargo clean` unless asked; `target/` is large but saves rebuild time.
