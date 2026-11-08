# Version control + AI chat panel — design decisions

Written 2026-11-08. This is a decision document, not an implementation. Nothing here is
built yet. Read it, change what you disagree with, then we build.

---

## Part 1 — Version control

### Current state

- **There is no git repository.** `git status` fails with "not a git repository".
- The tree is **16 GB**, of which `target/` is essentially all of it.
- `.gitignore` already exists (inherited from the upstream fork) and already excludes the
  things that would otherwise be a disaster:

  ```
  /target          (16 GB of build output)
  /dist            (52 MB of DMG + zip)
  /assets/models   (12 MB of ONNX OCR models)
  plan/            (private observation notes)
  .claude/
  ```

- `.gitattributes` also already exists.
- No secrets in the tree. The one `BEGIN PRIVATE KEY` hit is
  `crates/sign/tests/it/crypto.rs:282`, which is a **malformed test fixture string**, not a
  key. Safe to commit.

So the repo is already in good shape for a first commit. Two things need adding.

### What to add to `.gitignore`

```
/.perch          Perch's own folder index — 953 KB, machine-specific, regenerated
```

That is the only real gap. `.DS_Store` is already covered.

### Setup

```bash
cd "/Users/shyam/Downloads/New Project Of Mine/pdfcraft-main"

git init -b main
git add -A
git commit -m "Folio 0.18.1: initial commit

Fork of PdfCraft (https://github.com/storytold/pdfcraft) redesigned as a
distraction-free reader. Monochrome chrome, two-row shell, split view with
per-pane tools, horizontal position lock, OCR, Folio branding."
```

`git add -A` on this tree is safe because `target/` is ignored. Expect roughly 2,000 files
and a few tens of MB — the source, assets, icons, translations and test fixtures.

### Remote

**Recommendation: private GitHub repo.** This is a rebranded fork carrying your name and
Telegram handle. Keep it private until you have decided whether to publish. If you never
publish, a local repo plus `git bundle` to an external drive is enough.

```bash
git remote add origin git@github.com:<you>/folio.git
git push -u origin main
```

### Branching

You are solo, so keep it boring:

- `main` — always builds, always passes the 1427 integration tests.
- Short-lived feature branches: `feat/ai-chat`, `fix/hscroll-lock`.
- Tag releases: `git tag -a v0.18.2 -m "0.18.2"`.

Do not introduce a develop branch or release branches. One person does not need them.

### Two documents go stale

`HANDOFF.md` says, at line 14:

> There is **no git repository** here. Edit in place; do not expect `git diff`, branches or a
> clean checkout.

That becomes false the moment you run `git init`. Update it in the same commit, and drop the
"do not expect `git diff`" instruction. `AGENTS.md` has no equivalent claim, so it needs no
change.

### Why this matters before the feature

The AI chat work touches `canvas/selection.rs`, `app.rs`, `split.rs`, `dialogs/preferences.rs`
and adds at least one new crate. That is exactly the kind of change where you want to be able
to `git diff` and `git revert`. Do this first, in its own commit, before a single line of
feature code.

---

## Part 2 — The AI chat panel

### The hard constraint you need to know first

I checked the dependency tree. This codebase has:

| Needed for an embedded chat site | Present? |
|---|---|
| Webview (`wry`, `tao`, `WKWebView`) | **No** |
| HTTP client (`ureq`, `reqwest`) | **No** |
| Async runtime (`tokio`) | **No** |

The app is a pure egui + wgpu shell. The only network code it has ever had is the update
check, which went through the external `craft-update` crate and is now unreachable.

This matters because your spec says "integrate normal chats like chat.openai.com or gemini,
like a mini window browser for chatbots, if they can login". That is a **webview** feature,
and a webview is a fundamentally different and much larger piece of engineering than a chat
panel that calls an API.

### The webview route is a trap. Here is why.

1. **OpenAI and Google both block login inside embedded webviews.** They detect the webview
   user agent and refuse the sign-in flow. Google blocks OAuth in embedded webviews as a
   matter of policy, not as a bug. You would ship a login screen that cannot log in.
2. **It is against their terms.** Automating or wrapping a consumer ChatGPT session to feed it
   document text is not what those accounts are for. Accounts get flagged for it.
3. **It breaks the product promise.** The handoff says: *"Local/private: no account or
   telemetry."* A webview pointed at ChatGPT is a logged-in third-party session inside a
   privacy-first PDF reader. That is a different product.
4. **It is a large dependency with a large blast radius** — a whole browser engine, its
   entitlements, its cookie store, its update cadence, and its own security surface.

**Recommendation: do not build the webview.** Build the API panel.

### What I recommend instead

**A provider registry that speaks the OpenAI-compatible chat-completions protocol, plus two
extra adapters.**

One protocol gets you most of the market, because "OpenAI-compatible" is now the de facto
standard:

- OpenAI
- OpenRouter (which itself fronts Anthropic, Google, Meta, Mistral, and dozens more)
- Groq, Together, Fireworks
- **Ollama, LM Studio, llama.cpp server — local, on your own machine**

Then two adapters for the APIs that genuinely differ:

- Anthropic (Messages API)
- Google (Gemini API)

That is three adapters total, and it covers everything you listed. The local option is the
one I would lead with: a privacy-first PDF reader that can send a selection to a model running
on the same machine is a coherent product. Sending it to OpenAI is a different, weaker promise.

### What this requires building

- **An HTTP client.** `ureq` with `rustls` is the right fit: blocking, small, no async runtime,
  no `unsafe`, and it matches how the app already does one-shot HTTPS. It also keeps the
  "no tokio" property, which matters because egui is a single-threaded immediate-mode loop.
- **A worker thread per request.** The UI must never block on a network call. Send the request
  on a thread, poll a channel each frame, repaint when a chunk arrives. This is the same shape
  as the existing render worker in `crates/render`.
- **Streaming.** A chat that waits eight seconds and then dumps a wall of text feels broken.
  Stream the response (SSE for the hosted APIs, chunked for local). This is the single biggest
  quality difference between a demo and something you would actually use.
- **Key storage in the OS keychain** — macOS Keychain, Windows Credential Manager, libsecret
  on Linux. **Not** a JSON settings file. The app already has a `pdfcraft-platform` crate for
  exactly this kind of OS service; this belongs there.

### Layout: the decision you asked about

Your proposal was: single mode → PDF slides left, chat docks right. Split mode → chat floats
over the other pane.

**I would change one thing: do not float the chat over a PDF.**

A floating panel over a document you are reading covers the text, and it raises the question
you already spotted — which pane does it belong to? Floating is the wrong answer to that
question.

**Make the chat a property of the document, not of the pane.**

This is already the pattern the codebase uses everywhere:

- `DocView.tool` + `tool_owner` — each document remembers its own tool.
- `DocView.hscroll_lock` — per-view, not global.
- Split panes each keep their own state.

Follow it. Each `DocView` gets its own conversation. The chat panel is **one** panel, docked
in **one** place, and its *content* follows the focused document.

So, answering your question directly:

| Situation | What happens |
|---|---|
| Single PDF, select text | Chat docks right, PDF refits to the remaining width |
| Split, select in **left** PDF | Chat shows the **left document's** conversation |
| Split, select in **right** PDF | Chat shows the **right document's** conversation |
| Switch focus between panes | Chat content swaps; the panel does not move |

The panel never jumps sides. It never covers a page. There is no ambiguity about ownership,
because ownership is the document, and the document is already tracked.

### Width rules

The window is ~1470 pt in your current setup.

- **Single mode:** PDF gets `width - 380`, chat gets 380. Comfortable.
- **Split mode:** two panes plus a 380 pt dock leaves ~545 pt per PDF. Cramped but readable at
  fit-width. This is the honest cost of three columns.
- **Below ~1100 pt window width:** three columns stop working. At that point the chat becomes
  an overlay on the **focused** pane only, with a close cross. This is the one case where
  floating is correct, because there is no room for anything else.

The "PDF slides left and refits" part is nearly free: `Fit::Width` already recomputes from
`avail.width()` every frame, so opening the dock refits automatically. One thing to watch —
if `hscroll_lock` is on, changing the dock width changes the offset clamp, so the lock and the
dock need a test together.

### The selection bar: horizontal, icons only

**Decision (2026-11-08): the bar stays horizontal, and every action is an icon, not a text
label.** The user rejected short text labels ("Highlight", "Strikethrough", "Underline", …)
because they make the bar too wide; a row of 24 px icons takes far less space and reads faster.

The bar appears under a text selection. A text selection is wide and short — a line or two of
prose — so a horizontal bar sits under it in space that is already empty. A vertical bar has to
sit to one side and overlaps the lines above or below, which is the text you are about to read.
Keep it horizontal.

Each action is a Lucide icon (from `crate::icons`, the same set the toolbar uses), with the
English label as its tooltip and accessibility name:

| Action | Icon | Enabled when |
|---|---|---|
| Search the web | `search` | always (opens the selected text with a configured web action) |
| Add comment | `message-square-plus` | the document can take comments |
| Highlight | `highlighter` | the document can take comments |
| Underline | `underline` | the document can take comments |
| Strikethrough | `strikethrough` | the document can take comments |
| Replace text | `replace` | the document can take comments |
| Copy | `copy` | the selection can be copied |

"We can't" actions still draw, greyed, so the bar keeps its shape and the controls stay
discoverable.

**Search the web** opens the selected words in the system browser through `Context::open_url`
(the app's `open_url` path; the `webbrowser` crate is already in the tree). It closes the bar but
keeps the words selected, exactly like Copy, so a second action can follow.

The sites are **configurable** in Preferences ▸ Search the web: each is a name and a URL template
with a `{query}` placeholder (for example `https://chatgpt.com/?q={query}`). The defaults are
Google, DuckDuckGo, Bing, ChatGPT, Gemini and Perplexity. With one action configured the search
icon opens it directly; with several it opens a menu, so every engine and AI tool is one click
away. The query is percent-encoded by a small local helper — no new dependency — and the list is
stored in the app's local settings with the other preferences. This is also why no built-in
chatbot is needed: any AI tool that accepts a query in its URL can be added here.

This is implemented in `crates/ui-egui/src/webtools.rs` (the list and encoding),
`crates/ui-egui/src/canvas/selection.rs` (`selection_actions`) and
`crates/ui-egui/src/dialogs/preferences.rs` (`card_web_tools`).

Adding the button inherits all the existing positioning, dismissal and accessibility behaviour
from the current bar for free.

### Changing a mark's colour

Clicking an existing highlight, underline or strikeout with the Select tool selects it and opens
a compact **colour panel** beside the mark, so its colour can be changed without the full Comment
Properties dialog. Picking a swatch is one `Edit::StyleAnnotation` (one undo step); the panel stays
open while you try colours and closes when the mark is deselected. Implemented in
`crates/ui-egui/src/canvas/selection.rs` (`markup_colors`). The full dialog (opacity, thickness,
author, review history) remains on the comment's right-click menu as **Properties…**.


### Saving a conversation

You asked what the best way is. There are two, and they answer different needs:

1. **Attach it to the document as a PDF comment on the page.** It travels with the file, it is
   visible in any reader, it survives sharing, and it sits exactly where the reading happened.
   This is the durable artifact.
2. **Export to Markdown** next to the PDF (`report.chat.md`). Greppable, diffable, pasteable
   into anything, and it survives the app being uninstalled. This is the escape hatch.

Do both. Images go in a `<name>.chat/` folder beside the PDF and are referenced from the
Markdown.

Do **not** invent a proprietary format. There is no version of this where a custom binary blob
is the right answer.

### What is missing from your spec

You asked what you are missing. This is the list, roughly in order of how much it will hurt.

**Blocking**

1. **No webview, no HTTP, no async** — covered above. This is the one that changes the plan.
2. **Privacy promise conflict.** The app currently promises no account and no telemetry. Sending
   a selection to a third-party API is the opposite. This needs an explicit, per-provider opt-in
   and a visible indicator when text leaves the machine. Do not bury it in settings.
3. **API key storage.** Keychain, not JSON. A plaintext key in
   `~/Library/Application Support/` is a real vulnerability, not a nitpick.

**Will bite during the build**

4. **Two selection bars in split view.** The bar is keyed by `view.id`, so both panes can draw
   one. Only the focused pane's should show, or they will overlap and both will be clickable.
5. **`actions_dismissed` interaction.** The existing dismissal logic keys on the selection
   itself. A new "Ask AI" action has to respect it, or the bar will reappear after being closed.
6. **Token limits.** A long selection plus a long conversation will exceed the context window.
   You need a budget, a truncation rule, and a visible warning — not a silent failure.
7. **Streaming failure mid-response.** The connection will drop. The partial answer must be
   kept and marked incomplete, not thrown away.
8. **Cost visibility.** Users on a paid key will want to know what a conversation cost. At
   minimum, show token counts.
9. **i18n.** The app is fully translated through `craft-i18n`. Every new string needs a catalog
   entry, or it ships in English only.
10. **Accessibility.** The app leans heavily on AccessKit. The chat panel needs proper roles,
    labels and focus order, and the new button needs one too.

**Process, but enforced**

11. **The house rules apply.** `AGENTS.md` forbids `unwrap`, `expect`, `panic!` and `unsafe`
    outside tests, and clippy enforces it. New code follows that.
12. **Attribution.** A new HTTP crate needs an `ATTRIBUTION.toml` entry, and
    `cargo xtask assets` must pass. That check is part of CI.
13. **Tests.** 1427 integration tests currently pass. New UI needs coverage in the same style,
    including the control channel (`ui.*`) so it can be driven headlessly.
14. **The control channel.** New panels should be inspectable and clickable through `ui.inspect`
    and `ui.click`, or you lose the ability to test this feature the way we tested the
    horizontal lock.

### Suggested build order

Each phase is independently useful and independently testable. Do not start phase 2 before
phase 1 is merged.

| Phase | Work | Why this order |
|---|---|---|
| 0 | `git init`, first commit, private remote, update `HANDOFF.md` | You want to be able to revert before touching anything |
| 1 | Provider registry + keychain storage + settings card | No UI yet; the data model and secret handling are the risky parts |
| 2 | HTTP worker + one adapter (OpenAI-compatible) + streaming, headless | Prove the transport before drawing anything |
| 3 | Chat panel docked right, single mode only, Ask AI button on the selection bar | The core loop, one layout case |
| 4 | Per-document conversations + split mode + the narrow-window overlay rule | The layout question you asked about |
| 5 | Save as comment, export to Markdown, images | The output |
| 6 | Anthropic and Google adapters, local Ollama preset | Breadth, once the shape is proven |

Phase 2 is the one to be strict about. If streaming and cancellation work headlessly, the UI
is a day's work. If they do not, no amount of panel polish saves it.

---

## Decisions I need from you

1. **Transport:** API panel (recommended) or webview? I have argued against the webview; if you
   want it anyway, say so and I will scope it honestly rather than talk you out of it twice.
2. **Remote:** private GitHub, or local-only?
3. **Selection bar:** horizontal with an added Ask AI button (recommended), or vertical icons?
4. **Split-mode chat:** per-document content in one fixed panel (recommended), or one
   conversation shared across both PDFs?
5. **Saving:** PDF comment + Markdown export (recommended), Markdown only, or comment only?
6. **First provider:** OpenAI-compatible generic, or start with local Ollama to keep the privacy
   promise intact?
