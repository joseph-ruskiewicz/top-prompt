# top-prompt: AI chat components for Topcoat

## Intent

A set of copy-to-own AI chat components for [Topcoat](https://github.com/tokio-rs/topcoat)
(tokio-rs's server-rendered Rust framework), shipped the way Topcoat's own UI
components ship: as a **custom `topcoat ui` registry crate**. An app adds the
crate as a dependency and runs `topcoat ui add message --registry top-prompt`.

The components should look like they belong to the built-in registry: same
conventions, same neutral theme, no new design tokens.

## Reference material (read before writing code)

A full Topcoat checkout lives at `~/Projects/topcoat` (v0.12.1, same as
crates.io). Treat it as **read-only**. Start with:

| What | Where |
|---|---|
| Framework overview for agents | `llms.txt`, `AGENTS.md` |
| UI registry model, custom registries, theming | `docs/ui.md` |
| Component conventions (copy these exactly) | `crates/topcoat-ui/registry/src/components/{button,textarea,accordion,dialog,tooltip,avatar,badge,spinner}.rs` |
| Registry crate setup | `crates/topcoat-ui/registry/{Cargo.toml,build.rs,registry.toml,src/lib.rs}` |
| Theme tokens | `crates/topcoat-ui/registry/themes/neutral.css` |
| Showcase app + registry sync test | `examples/ui/` (incl. `tests/registry_sync.rs`) |
| Streaming regions, WebSocket renders | `docs/view/live.md`, `examples/live/src/chat.rs` |
| Signals, `$()` expressions, `raw!`, procedures, shards | `docs/runtime.md`, `docs/runtime/*.md`, `examples/runtime/` |
| Registry loading / cross-registry deps | `crates/topcoat-ui/src/registry.rs`, `crates/topcoat-ui/src/manage/` |

Where this plan and the Topcoat source disagree, the source wins; note the
deviation in the final report.

## Workspace layout

```
top-prompt/
  Cargo.toml                    # workspace: crates/top-prompt, examples/showcase
  rust-toolchain.toml           # stable
  README.md                     # install + usage + component list
  crates/top-prompt/            # the registry crate
    Cargo.toml                  # [package.metadata.topcoat-ui] registry = "."
    build.rs                    # stage lucide icons behind a `stage-icons` feature
    registry.toml               # version = 1, [components.*], no themes
    src/lib.rs                  # #[cfg(all(test, feature = "stage-icons"))] pub mod components;
    src/components.rs
    src/components/*.rs         # one file per component
  examples/showcase/            # runnable gallery + working demo chat
    Cargo.toml  build.rs  styles.css  components.toml
    src/main.rs  src/components/*.rs (installed via `topcoat ui add`)
    tests/registry_sync.rs
```

Mirror `topcoat-ui-registry` for the registry crate: component sources compile
only as test modules, so the crate has no runtime code of its own. Because the
sources `use` built-in components (`super::button::...` after install), work out
how to make them compile in the registry crate's test build — the simplest
route is probably to rely on the showcase as the compile check and keep the
registry crate source-only. Pick whichever works and keep it simple.

Depend on `topcoat = "0.12.1"` from crates.io (features as `examples/ui` uses:
`font-fontsource`, `icon-iconify`, `tailwind`, `ui`).

## Hard rules

1. **Neutral theme only.** Use the existing token utilities (`bg-background`,
   `text-foreground`, `bg-card`, `text-muted-foreground`, `bg-primary`,
   `text-primary-foreground`, `border-border`, `ring-ring`, `bg-destructive`,
   `bg-popover`, `shadow-xs/sm`) and opacity derivations of them
   (`bg-foreground/5`, `hover:bg-foreground/10`) exactly as built-in components
   do. No raw colors, no new CSS variables, no new theme, no edits required to
   the user's `styles.css`. That means no custom `@keyframes`: animations use
   Tailwind's built-ins (`animate-pulse`, `animate-bounce`, `animate-spin`) and
   transitions.
2. **Reuse built-ins, don't restyle them.** Buttons are `button`/`button_variants`,
   avatars are `avatar`, chips are `badge`, spinners are `spinner`, tooltips are
   `tooltip`, etc. Declare them as cross-registry dependencies
   (`{ registry = "topcoat-ui-registry", name = "button" }`) so `topcoat ui add`
   pulls them in.
3. **Built-in conventions.** `#[component] pub async fn name(cx: &Cx, ...)`,
   compound parts as separate functions (`message`, `message_content`, ...),
   enum props for variants/sizes with `#[default]`, `#[default] mut attrs: Attributes`
   forwarded to the documented root element with `class!(BASE, attrs.remove("class"))`,
   children as `Child<'_>`, `StaticClass` constants, doc comments with an
   `ignore` example on every public item, Lucide icons via `iconify_icon!`.
4. **No hand-written JS files and no WASM.** Interactivity uses native HTML
   first (`<details>`, `<form>`, `field-sizing-content`, CSS `:has`/`group-*`),
   then the Topcoat runtime (`signal`, `$()`, `@event`, `raw!`). Each component
   should degrade sensibly without the runtime where practical.
5. **Components are presentational.** They take data and children; they never
   call a model, own conversation state, or assume a provider. Wiring to an LLM
   is the app's job (the showcase demonstrates it with a fake model).
6. **Accessible.** Proper roles/labels (`role="log"` + `aria-live="polite"` on
   the conversation, `aria-label` on icon buttons, `aria-busy` while streaming),
   visible focus rings matching built-ins, keyboard-operable everything.
7. Works in light and `.dark` with no component changes.

## Components

Registry names are the file names. Parts listed are the public functions.

| # | Component | Parts | Behavior |
|---|---|---|---|
| 1 | `conversation` | `conversation`, `conversation_content`, `conversation_empty_state`, `conversation_scroll_button` | Scrollable column, `role="log"`. Sticks to the bottom while content streams unless the user has scrolled up; scroll button appears when not at bottom. Prefer a CSS-only stick-to-bottom (e.g. `flex-col-reverse` or `overflow-anchor`) and fall back to `raw!` only if needed. Content is a centered `max-w-3xl` column. |
| 2 | `message` | `message` (prop `role: MessageRole::{User, Assistant, System}`), `message_avatar`, `message_content` (variant `Bubble`/`Flat`), `message_actions`, `message_action` | User: right-aligned bubble (`bg-foreground/5`). Assistant: left-aligned, flat, full width. Actions row is a ghost icon-button toolbar with tooltips, revealed on hover/focus-within (always visible on the last message and on touch). |
| 3 | `prompt_input` | `prompt_input` (a `<form>`), `prompt_input_textarea`, `prompt_input_toolbar`, `prompt_input_tools`, `prompt_input_button`, `prompt_input_submit` (prop `status: PromptStatus::{Ready, Submitted, Streaming, Error}`) | Rounded bordered card whose whole surface shows the focus ring via `focus-within`. Textarea auto-grows (`field-sizing-content`) up to a max height, borderless. Enter submits, Shift+Enter inserts a newline, IME composition is respected. Submit is an icon button: arrow-up when ready, spinner when submitted, square "stop" while streaming. Disabled when empty where feasible. |
| 4 | `markdown` | `markdown` (prop `source: &str`) | Server-side render with `pulldown-cmark` (GFM tables, strikethrough, task lists). **Raw HTML in the source is escaped, never passed through**; link `href`s are restricted to `http`, `https`, `mailto` and get `rel="noopener noreferrer" target="_blank"`. Styled with token utilities via descendant selectors on the wrapper (no typography plugin). Fenced code blocks render through `code_block`. Must tolerate incomplete markdown mid-stream (unclosed fence, half a link) without breaking layout. Document that the consuming app adds `pulldown-cmark` to its `Cargo.toml`. |
| 5 | `code_block` | `code_block` (props `language: Option<&str>`, `code: &str`), `code_block_header`, `code_block_copy` | Bordered `bg-card` block, header with language label and copy button, horizontally scrollable monospace body. No syntax highlighting in v1 (see non-goals). |
| 6 | `copy_button` | `copy_button` (prop `text`) | Ghost icon button; writes to the clipboard and swaps the copy icon for a check for ~2s. Used by `code_block` and as a `message_action`. |
| 7 | `reasoning` | `reasoning` (props `streaming: bool`, `duration_secs: Option<u32>`), `reasoning_trigger`, `reasoning_content` | Native `<details>` using the accordion's height animation. Open while `streaming` with a pulsing "Thinking…" label; closed afterwards showing "Thought for N seconds". Muted, smaller text with a left border. |
| 8 | `tool_call` | `tool_call` (props `name`, `status: ToolStatus::{Pending, Running, Completed, Error}`), `tool_call_header`, `tool_call_input`, `tool_call_output` | Collapsible bordered card. Header: wrench icon, tool name, status `badge` (spinner while running, destructive on error). Input and output are labelled monospace sections; output has an error style. |
| 9 | `sources` | `sources` (prop `count`), `sources_trigger`, `sources_content`, `source` (props `href`, `title`), `inline_citation` | Collapsible "Used N sources" list of links; `inline_citation` is a small numbered `badge`-style superscript link, with a `hover_card` preview if that stays simple. |
| 10 | `suggestion` | `suggestions`, `suggestion` | Horizontally scrollable row (or wrapped grid) of outline pill buttons. Each is a `<button>` carrying its prompt in `value`/`data-*` so the app decides what a click does (the showcase submits it). |
| 11 | `loader` | `loader` (prop `variant: LoaderVariant::{Dots, Pulse, Text}`, optional label) | Typing indicator for the gap before the first token: three bouncing dots (staggered `animate-bounce` delays), a pulsing dot, or pulsing text like "Thinking…". `role="status"`. |
| 12 | `attachment` | `attachments`, `attachment` (props `name`, `media_type`, optional `preview_src`, `removable`), `attachment_remove` | File chips for the prompt input and for sent messages: image thumbnail or file-type icon, truncated name, remove button. Presentational only; no upload logic. |
| 13 | `branch_picker` | `branch_picker`, `branch_picker_previous`, `branch_picker_next`, `branch_picker_count` (props `current`, `total`) | "‹ 2 / 3 ›" control for alternate responses, sized to sit in `message_actions`. |

Build order: `copy_button`, `loader`, `message`, `conversation`, `prompt_input`,
`code_block`, `markdown`, `reasoning`, `tool_call`, `sources`, `suggestion`,
`attachment`, `branch_picker`. After the first three exist, stand up the
showcase and keep it compiling as each one lands.

## Showcase app (`examples/showcase`)

Set up like `~/Projects/topcoat/examples/ui`: `topcoat ui init --theme neutral`,
then install components with the real CLI (`topcoat ui add --all --registry
top-prompt`, plus the built-ins they depend on) so the install path is the thing
being tested. Geist via Fontsource. A dark-mode toggle.

Two pages:

- **`/` Gallery** — every component and every variant/state, statically
  rendered with realistic content, each under a heading with its registry name.
- **`/chat` Demo** — a working chat built from the components, following the
  `examples/live/src/chat.rs` pattern (shared state in router context, a
  `#[procedure]` to send, a `#[shard]` with `live!` + `connected(cx)` to
  stream). The "model" is a **fake**: a Tokio task that streams a canned
  response chunk by chunk with delays — a reasoning phase, a tool call that
  goes Running → Completed, then markdown text with a code block and sources.
  No API keys, no network. Stop button cancels the stream. Suggestions on the
  empty state. The generation runs in a spawned task and the live region only
  observes it, so reconnects don't restart it.

`tests/registry_sync.rs`: adapted from the Topcoat example — every component in
`crates/top-prompt/registry.toml` is installed in the showcase and byte-identical
to the registry source.

## Verification (all must pass before reporting done)

- `cargo build --workspace` and `cargo clippy --workspace --all-targets` clean.
- `cargo fmt --check` and `topcoat fmt --check`.
- `cargo test --workspace` (registry sync, plus unit tests for the markdown
  renderer: raw HTML is escaped, `javascript:` links are dropped, unclosed code
  fence renders).
- In a scratch app outside the workspace: `topcoat ui add message --registry
  top-prompt` installs the component and its built-in dependencies, and the app
  builds.
- Run the showcase and check it in a real browser, light and dark: gallery
  renders every component; in `/chat`, send a message and watch reasoning, tool
  call, and markdown stream in; stick-to-bottom holds, scrolling up releases it;
  Enter / Shift+Enter behave; copy and stop work; no console errors.

## Non-goals (v1)

- Syntax highlighting, LaTeX, mermaid.
- Any real LLM/provider integration, persistence, or auth.
- Voice input, file upload handling, message editing UI.
- A new theme or theme extensions.
- Publishing to crates.io, git commits (leave the tree uncommitted).

## Report back

List what was built, each place the implementation deviates from this plan and
why, the exact verification commands run with their results, and anything that
is known not to work.
