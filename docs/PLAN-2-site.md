# top-prompt phase 2: prompt-kit-influenced components and a docs website

Builds on `docs/PLAN.md` (done). All of its hard rules still apply, with the
one amendment in "Animations" below.

Reference: https://www.prompt-kit.com/ — look at the home page, a component
page such as `/docs/prompt-input`, and `/blocks` in a browser before starting.
We take its **component vocabulary, page structure and visual restraint**. We do
not take its name, logo, copy, example text, or code: everything is written
fresh for Topcoat, in the neutral theme.

## Part A — components

### New components

| Component | Parts | Notes |
|---|---|---|
| `text_shimmer` | `text_shimmer` (props `duration_ms`, `spread`) | Muted text with a highlight sweeping across it. Respect `prefers-reduced-motion`. |
| `thinking_bar` | `thinking_bar` (props `text` default "Thinking", optional `stop_label`) | Shimmering label plus an optional inline "Answer now"-style button; the app wires the click. |
| `steps` | `steps` (`default_open`), `steps_trigger`, `steps_content`, `steps_bar`, `steps_item` | Collapsible list of agent operations with a vertical bar down the left. Trigger takes an optional leading icon. |
| `chain_of_thought` | `chain_of_thought`, `chain_of_thought_step`, `chain_of_thought_trigger`, `chain_of_thought_content`, `chain_of_thought_item` | A sequence of individually collapsible reasoning steps joined by a connector line (last step has none). |
| `feedback_bar` | `feedback_bar` (props `title`), `feedback_bar_helpful`, `feedback_bar_not_helpful`, `feedback_bar_close` | "Was this response helpful?" strip with thumbs up/down and dismiss. Buttons carry `value`s; the app wires them. |
| `system_message` | `system_message` (prop `variant: Action/Warning/Error`, `fill: bool`, optional icon), `system_message_cta` | Inline notice in the conversation. Error uses `destructive`; Warning has no dedicated token, so derive it from `foreground` at reduced opacity plus a warning icon rather than inventing a color. |
| `file_upload` | `file_upload` (props `multiple`, `accept`, `name`), `file_upload_trigger`, `file_upload_content` | Wraps a real `<input type="file">`. Trigger opens the picker; dropping files on the zone assigns them to the input; `file_upload_content` is the overlay shown while dragging. No upload logic. Pairs with the existing `attachment` chips. |
| `image` | `image` (props `src` or `base64` + `media_type`, required `alt`) | Generated-image display: rounded, bordered, responsive, with a skeleton-like placeholder while loading. |

### Changes to existing components

- `loader`: add variants so it covers the common set — keep `Dots`, `Pulse`,
  `Text`; add `Circular` (the built-in `spinner`), `Typing`, `Wave`, `Bars`,
  `TextShimmer` (reuses `text_shimmer`), and sizes `Sm/Md/Lg`.
- `suggestion`: add a highlight mode — a `highlight: Option<&str>` prop renders
  the suggestion as a full-width ghost row with the matching substring in
  `text-foreground` and the rest muted (for autocomplete-style lists).
- `sources`: add a compact chip form — `source_chip` (props `href`, `label`,
  `show_favicon`) with a hover card showing title and description. Keep the
  existing collapsible list.
- `code_block`: accept pre-rendered child content as an alternative to the
  `code` prop, so an app can pass highlighted markup. The component itself stays
  unhighlighted and dependency-free.
- `reasoning`: use `text_shimmer` for the streaming label.

### Animations (amendment to hard rule 1)

Shimmer, wave and bars need keyframes Tailwind does not ship. A component may
emit its own small `<style>` element containing `@keyframes` (and nothing but
keyframes and a `prefers-reduced-motion` guard). Colors still come only from
theme tokens. The user's `styles.css` still needs no edits.

## Part B — the website

A docs site for top-prompt, **built with Topcoat and the top-prompt components
themselves**, laid out very close to prompt-kit's.

### Where it lives

New workspace member `site/`. It replaces `examples/showcase`: move the working
demo chat and the registry-sync test into it and delete the showcase, rather
than keeping two apps in sync. Components are installed into `site/` with the
real `topcoat ui add` flow, as before.

### Layout (match prompt-kit closely)

- **Fixed left sidebar**, ~190px, separated from the content by a hairline
  (theirs is dashed). Top: a small logo mark and the wordmark "top-prompt".
  Then grouped link lists with small muted group labels:
  - *Get Started*: Home, Introduction, Installation
  - *Components*: every component, alphabetical, with a small muted "new" tag
    on the ones added in this phase
  - *Blocks*
  - *Resources*: llms.txt, GitHub (link target is a constant placeholder)
  - a light/dark theme toggle at the bottom

  The current page gets a subtle filled pill (`bg-foreground/5`). Small text
  (13–14px), generous vertical rhythm. On narrow screens the sidebar collapses
  into a sheet behind a menu button.
- **Content column**: a single narrow column (~640px) offset from the sidebar
  with lots of whitespace, not centered full-width. Blocks pages use a wider
  column (~800px).
- Typography is quiet: page titles ~20px semibold, body 14px, no hero-sized
  type, no gradients, no shadows beyond the theme's.

### Pages

- **Home (`/`)**: two-line headline where the second line is muted
  ("…building blocks for AI apps in Rust." / a muted descriptive line), two
  buttons (primary "Get Started", outline "GitHub"), then a bordered preview
  panel containing a live `prompt_input` with tool buttons, then a code panel
  showing the Rust that produced it, then a "next page" link bottom-right.
- **Introduction, Installation**: prose. Installation covers adding the crate,
  `topcoat ui init`, `topcoat ui add <name> --registry top-prompt`, the icon
  staging in `build.rs`, and per-component extra crates (`pulldown-cmark`).
- **One page per component (`/docs/<name>`)**, all with the same anatomy:
  1. Title and one-line description.
  2. **Examples** — each has a heading, optional sentence, and a
     **Preview | Code** tab pair. Preview is a bordered panel (min-height
     ~350px) with the live component centered in it. Code is the example's
     Rust source with a copy button.
  3. **Installation** — **CLI | Manual** tabs. CLI shows the `topcoat ui add`
     command with copy. Manual lists extra dependencies and shows the
     component's full source.
  4. **Component API** — one table per public function: Prop, Type, Default,
     Description.
  5. Previous / next links.
- **Blocks (`/blocks`)**: larger composed examples, each with Preview | Code and
  an "open full page" link: prompt input with actions, prompt input with
  suggestions, prompt input with autocomplete highlight, basic conversation,
  conversation with avatars, conversation with actions, conversation with
  scroll-to-bottom, conversation with prompt input, sidebar with chat history,
  and the full chat app (the existing fake-model streaming demo).
- **`/llms.txt`**: plain-text index of the docs for agents.

### Single source of truth

Docs must not drift from the code:

- Each example is a real `#[component]` in its own file; the Preview renders it
  and the Code tab shows that same file via `include_str!`.
- Manual-install source is read from the registry crate's files.
- Describe each component once in a small data structure (name, description,
  examples, API rows, "new" flag) that drives the sidebar, the page, prev/next
  and `llms.txt`. A test asserts every component in `registry.toml` has a docs
  page and at least one example.

### Code highlighting

The Code tabs should be syntax-highlighted Rust, as on the reference site.
Highlight server-side in the site (not in the registry component) and pass the
markup into `code_block`. Token colors are site-only CSS in `site/styles.css`,
kept to a restrained palette that works in light and dark. Pick the smallest
dependency that does the job.

## Verification

Same bar as phase 1: workspace build, clippy, `cargo fmt --check`,
`topcoat fmt --check`, `cargo test --workspace`, the scratch-app install of one
new component, and a real-browser pass in light and dark covering: home, two
component pages (tabs switch, copy works, API tables render), `/blocks`, the
full chat demo still streaming, the mobile sidebar sheet at a narrow width, and
no console errors. Compare the home page and one component page side by side
with prompt-kit and note where the layout differs.

Not in scope: deploying the site, a search box, analytics, git commits.

## Report back

As in phase 1: what was built, deviations and why, each verification command
with its actual result, and what was not verified.
