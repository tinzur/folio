<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/ai.storyteller.pdfcraft.png" width="128" height="128" alt="Folio app icon: a white page on a dark rounded tile">
</p>

<h1 align="center">Folio</h1>

<p align="center">
  <b>A quieter PDF reader.</b><br>
  Distraction-free, local, and monochrome — built on the open-source PdfCraft engine.
</p>

<p align="center">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-555555?style=flat-square">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-555555?style=flat-square">
  <img alt="No account, no telemetry" src="https://img.shields.io/badge/no%20account-no%20telemetry-555555?style=flat-square">
</p>

<p align="center">
  <a href="https://t.me/Tinzur"><b>Telegram — @TINZUR</b></a> ·
  <a href="https://github.com/storytold/pdfcraft">PdfCraft source</a>
</p>

<br>

<p align="center">
  <img src="docs/images/folio-home.png" alt="Folio's minimal home page: Open PDF, optional actions, recent files and developer credit" width="100%">
  <br>
  <sub>The home page: one clear action, optional sections, and no account.</sub>
</p>

## What this is

Folio is an independent, distraction-free reading shell for PDFs, redesigned by
[TINZUR](https://t.me/Tinzur) on top of the open-source
[PdfCraft](https://github.com/storytold/pdfcraft) project.

The PDF engine, rendering, editing and file handling come from PdfCraft
(MIT OR Apache-2.0). The minimal chrome, monochrome theme, two-row tool layout,
focus mode and split view are this redesign.

It is **not** an ArtCraft distribution, and it is not affiliated with or endorsed
by ArtCraft. The original project is credited because its engine and application
are the foundation of this reader.

## The redesign

- **Read first.** The page is the interface. Tabs and one compact tool row, no
  navigation rail, no permanent catalogue.
- **Monochrome.** A calm black, white and grey palette in both light and dark.
- **Two rows.** Tabs on top; the annotation row below. It fits to the window,
  wraps to at most two rows, and can be customised.
- **Split view.** Two PDFs side by side, each with its own tool and scroll
  position, without a second title bar.
- **Focus mode.** Hide the annotation row while reading; bring it back at the
  top edge or with the eye button.
- **Horizontal position lock.** With mixed page widths, a lock beside the page
  and zoom controls keeps the same horizontal position when moving between
  pages. It is per pane, so it also works independently in split view.
- **Local and private.** No account, no telemetry, no cloud dependency.
- **Still a PDF editor.** Organise, combine, split, annotate, fill and sign,
  protect, and save, using PdfCraft's engine.
- **Scan & OCR included.** The OCR models (Robert Knight's OCrs models,
  CC-BY-SA-4.0, unmodified) ship with the app together with their full licence
  and attribution.

## Home

Home is intentionally small:

- **Open PDF** and drag-and-drop.
- Optional **Actions**, **Pinned** tools/folders, **Recent** files, and the
  developer/source card.
- **Customize home** turns any section on or off, changes how many recent files
  are kept, and pins tools or folders.
- Pinned and recent sections appear only when they have something to show.

## Build

```bash
cargo build -p pdfcraft
target/debug/pdfcraft path/to/file.pdf
```

Requirements and platform setup are the same as PdfCraft's. The desktop crate is
still called `pdfcraft`; the display name is Folio.

## Credits and links

- **Redesign and maintenance:** TINZUR — [t.me/Tinzur](https://t.me/Tinzur)
- **Original project:** PdfCraft — [github.com/storytold/pdfcraft](https://github.com/storytold/pdfcraft)
- **Engine:** hayro (bootstrap), egui, Lucide icons, Inter / JetBrains Mono / Dancing Script fonts.
- **Licence:** MIT OR Apache-2.0. See `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`
  and `ATTRIBUTION.md`.

The app contains no ArtCraft community buttons, website links, Discord links or
update prompt. It never checks for updates.
