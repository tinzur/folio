# Folio fork guide — what this fork changed, and what it did not

Written 2026-11-09. **Read this before `HANDOFF.md` if you are new to the project.**

`HANDOFF.md` tells you how to work here. This file tells you *which parts of the codebase are
ours*. That distinction matters: the tree is ~399k lines of Rust across 33 crates, and the
overwhelming majority of it is upstream PdfCraft, unchanged. A new agent that does not know this
will spend hours reading engine code that has nothing to do with the redesign, or worse, "fix"
upstream behaviour that was never in scope.

---

## 1. The one-paragraph version

Folio is a **reading shell**, not a PDF engine. The engine, renderer, editor, forms, OCR,
signing and file handling are [PdfCraft](https://github.com/storytold/pdfcraft) (MIT OR
Apache-2.0), taken as a fork and left functionally intact. This fork replaced the *chrome*: a
two-row shell, monochrome theme, focus mode, split view, a tools drawer, a minimal home page, and
a selected-text action bar. If a bug is in how a page renders or a PDF is written, it is upstream
and probably not ours. If a bug is in how the window is arranged, it is ours.

---

## 2. Where the fork's own code lives

Almost all of it is in **`crates/ui-egui/`**, and within that, in the files where this fork's
shell lives (per `HANDOFF.md` §4; the fork's history begins at the initial commit, so git cannot
separate fork-created files from upstream ones — treat these as the shell's home, not as proof
they are new):

| File | Lines | What it is |
|---|---|---|
| `crates/ui-egui/src/chrome.rs` | 616 | Document/tab row: tabs, split lock, focus eye, active-tab accent |
| `crates/ui-egui/src/hotbar.rs` | 1274 | Annotation/tool row, reading controls (page, zoom, view, panels, settings) |
| `crates/ui-egui/src/drawer.rs` | 455 | Full-width tools drawer, workspace chips, boxed catalog tiles |
| `crates/ui-egui/src/home.rs` | 304 | Minimal home: Open, Actions, Pinned, Recent, Developer |
| `crates/ui-egui/src/split.rs` | 188 | Two-pane split view, focus, divider, scroll lock |
| `crates/ui-egui/src/webtools.rs` | 128 | Configurable web actions for selected text |

Plus these, which carry this fork's changes (the diff against the initial commit is the reliable
record of what was added after it):

| File | What changed |
|---|---|
| `crates/ui-egui/src/app.rs` | Frame order, per-document tool save (`tool_owner`), focus-mode decision |
| `crates/ui-egui/src/canvas.rs` | `DocView.hscroll_lock`, `markup_colour_panel`, `document_area` wiring |
| `crates/ui-egui/src/canvas/zoom.rs` | `content_width` + `layout` honour `hscroll_lock` |
| `crates/ui-egui/src/canvas/selection.rs` | Icons-only action bar, `SelectionBar`, `markup_colors` |
| `crates/ui-egui/src/dialogs/preferences.rs` | Card 7 "Search the web" (`card_web_tools`) |
| `crates/ui-egui/src/app/prefs.rs` | Persists `web_tools` |
| `crates/ui-egui/src/lib.rs` | `APP_NAME`, `web_tools` field, `pub mod webtools` |
| `crates/ui-egui/src/menus.rs` | Branding, removed ArtCraft entries and update check |
| `apps/pdfcraft/src/main.rs` | `set_process_name`, window title from `APP_NAME` |
| `apps/pdfcraft/Cargo.toml` | `objc2-foundation` for the process name |

Everything else — `crates/engine`, `cos`, `render`, `forms`, `edit`, `annot`, `ocr`, `sign`,
`print`, `redact`, `xfa`, `automation`, `cli` — is upstream. Read it when you need to understand
a behaviour; do not assume it is in scope to change.

---

## 3. Upstream documents that are still accurate

These describe the *engine* and are worth reading as-is. They were written by the PdfCraft
authors and this fork has not invalidated them:

- `docs/architecture.md` — crate layers (L0–L7), what each layer may depend on. Authoritative.
- `AGENTS.md` — the binding rules (asset policy, clean-room, no `unsafe`/`unwrap`, parity
  tracking). This fork inherits all of it. **It wins over anything else.**
- `CLAUDE.md` — day-to-day working instructions, quality gates.
- `docs/development.md`, `docs/releasing.md`, `docs/mcp.md`, `docs/localization.md`.

Documents in `docs/` that describe the **old** PdfCraft UI (`ui-parity.md`,
`target-app-parity.md`, `redesign-status.md`, `integration-20261030.md`) are historical. They
describe a shell that no longer exists. Do not treat them as current.

---

## 4. The fork's own documents

| File | Purpose | Status |
|---|---|---|
| `HANDOFF.md` | How to work here: build, test, gates, traps, file map. **Start at §8.** | Current |
| `docs/folio-fork-guide.md` | This file: what is ours vs upstream | Current |
| `docs/ai-chat-design.md` | Decision document for an AI chat panel | **Nothing built.** Six open decisions. |
| `README.md` | Public-facing description and attribution | Current |

---

## 5. What the fork actually added (feature list)

Each of these has tests; the test names are given so you can find the behaviour quickly.

**Chrome and shell**
- Two full-width rows: document/tab row (`chrome.rs`), annotation/tool row (`hotbar.rs`).
  Auto 1–2 rows, max 2.
- Monochrome chrome over Light or Dark; page content keeps its real colours.
- Focus mode (`view.focus_mode`, ⇧⌘F): hides the annotation row while reading; returns at the top
  edge or via the eye button. Tests: `ui::focus_mode_hides_the_annotation_row_and_returns_at_the_top`,
  `ui::focus_mode_hides_the_row_in_split_and_returns_at_the_top`.
- Tools drawer with workspace chips (Read/Edit/Convert/E-Sign) and boxed catalog tiles.
- Minimal home page with toggleable sections, persisted as `home_show_*`.

**Split view** (`split.rs`)
- Exactly two panes, draggable divider, both render, no duplicate title bars.
- Per-pane tool state via `DocView.tool` + `tool_owner`. Tests:
  `ui::horizontal_position_lock_is_per_split_pane`, `ui::split_scroll_lock_is_a_visible_toggle`.
- Third PDF leaves split but remembers the pair (`split_saved`); Combine/Compare restore it.
- **Overlay guard**: pane pre-focus is skipped while a drawer, palette or dialog is open. Removing
  this makes tools apply to the left PDF. Do not remove it.

**Horizontal position lock** (`DocView.hscroll_lock`, `canvas/zoom.rs`)
- Keeps the reader's horizontal offset when paging between pages of different widths.
- The bug it fixed: `content_width` sized the scroll content to the *shown* page, so on a narrow
  page there was nothing to scroll and the offset collapsed to zero. The toggle flipped a flag
  and did nothing. Now `content_width` uses the full document width while locked, and `layout`
  left-aligns pages at `SIDE` so a preserved offset lands on the same column.
- Test: `view::horizontal_lock_keeps_the_offset_across_mixed_page_widths`.

**Selected-text action bar** (`canvas/selection.rs`)
- Icons only, 24 px, English label as tooltip and accessibility name. Text labels were rejected
  as too wide.
- Order: Search the web, Add comment, Highlight, Underline, Strikethrough, Replace text, Copy.
  Unavailable actions still draw, greyed.
- Test: `text_select::selected_text_offers_search_the_web_in_the_action_bar`.

**Configurable web search** (`webtools.rs`, Preferences card 7)
- Each entry is a name plus a URL template containing `{query}`. Defaults: Google, DuckDuckGo,
  Bing, ChatGPT, Gemini, Perplexity. `MAX_TOOLS = 20`, at least one always kept.
- One entry opens directly; several open a menu. Query percent-encoded locally (no dependency).
- Persisted as `web_tools`. `restore` drops invalid entries and falls back to defaults.
- **This is why no built-in chatbot was needed**: any AI tool that takes a query in its URL can
  be added.

**Mark colour panel** (`canvas/selection.rs::markup_colors`)
- Clicking an existing Highlight / Underline / StrikeOut / Squiggly with Select opens a compact
  swatch grid beside it. One `Edit::StyleAnnotation` = one undo step. Locked marks ignored.
- Test: `comments::clicking_a_highlight_opens_a_colour_panel`.

**Branding**
- `APP_NAME = "Folio"` in `crates/ui-egui/src/lib.rs`. Window title, Home, About, menus, README,
  packaging use it. **The internal crate and binary stay `pdfcraft`.**
- Telegram `https://t.me/Tinzur`; QR at `assets/brand/tinzur-qr.png`.
- ArtCraft links, brand assets, Discord/website cards and the update prompt were removed. Do not
  add them back.
- `eframe::storage_dir("PdfCraft")` and the Windows `LOCALAPPDATA/PdfCraft` path are
  **deliberately unchanged**: they hold existing user settings. Only the display name is Folio.

---

## 6. Numbers, so you can sanity-check a claim

Measured 2026-11-09 on this tree:

| Thing | Value |
|---|---|
| Rust LOC (`crates` + `apps` + `xtask`) | ~399,000 |
| Library crates | 33 |
| Binaries | 3 (`pdfcraft`, `pdfcraft-cli`, `pdfcraft-web`) |
| Registered commands (`engine/src/commands.rs`) | 210 |
| Tool catalog groups (`engine/src/catalog.rs`) | 24 |
| Integration test files (`ui-egui/tests/it/`) | 68 |
| Integration tests passing | 1430 (1 ignored) |
| Library tests passing | 751 (1 ignored) |
| Tracked files / tracked size | 1907 / ~44 MB |

If a future claim says "the tests pass" but the count is far from 1430 + 751, something changed
and the claim needs checking.

---

## 7. What is genuinely unfinished

In priority order, as of 2026-11-09:

1. **AI chat panel decision** — `docs/ai-chat-design.md`. Nothing built. The webview route was
   rejected (OpenAI and Google block login in embedded webviews; it breaks the "no account, no
   telemetry" promise; it needs a browser engine this tree does not have). The recommended shape
   is an API panel with per-document conversations. **But** the web-search feature already covers
   "ask an AI about this text", so weigh whether the panel is still wanted at all.
2. **Final name** — "Folio" is a working name, not a decision. Changing it means `APP_NAME`,
   README, packaging strings and a rebuild.
3. **Add-text paragraph drag** — one ignored test,
   `editing::add_text_finishes_a_paragraph_drag_that_leaves_its_box`. A real content-tool
   regression. Do not delete it.
4. **Split polish** — per-pane PDF picker without a title bar, a dedicated close-pane control,
   persisting the split group across restarts (the pair is in-session only; ratio persists).
5. **Focus polish** — hide the document row and side panels in focus mode; add an idle delay.
6. **Final DMG** — only when asked. arm64-only today; universal needs the `x86_64-apple-darwin`
   target. Ad-hoc signing is fine locally; distribution needs Developer ID + notarization. The
   OCR models must stay in the bundle.

---

## 8. How to get productive in ten minutes

```bash
cd "/Users/shyam/Downloads/New Project Of Mine/pdfcraft-main"

git log --oneline | head -10     # what has been done
cargo build -p pdfcraft          # ~20 min cold, seconds warm
cargo test -p pdfcraft-ui-egui --lib        # 751 tests, ~6 s
cargo test -p pdfcraft-ui-egui --test it    # 1430 tests, ~30 s
```

Then read, in this order: `AGENTS.md` (rules) → this file (scope) → `HANDOFF.md` §8 (state and
traps) → the specific file you need to change.

To see the app with the right name in the macOS menu bar:

```bash
cp target/debug/pdfcraft /tmp/Folio && /tmp/Folio "/path/to/file.pdf"
```

The menu title is the executable's file name. `setProcessName` does not change it — that was
measured, see `HANDOFF.md` §4.6.

---

## 9. Attribution (do not remove)

The engine is PdfCraft by the PdfCraft contributors, MIT OR Apache-2.0. This fork's chrome is
TINZUR's. `NOTICE`, `ATTRIBUTION.toml` and `ATTRIBUTION.md` record every asset, font and
dependency; `cargo xtask assets` enforces it and fails the build if an entry is missing or a hash
does not match. Do not hand-edit `ATTRIBUTION.md` — generate it.
