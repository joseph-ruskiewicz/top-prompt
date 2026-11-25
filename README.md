# topcoat-prompt

AI chat components for [Topcoat](https://github.com/tokio-rs/topcoat): a conversation
log that sticks to the bottom while it streams, messages, a prompt input, server-rendered
markdown, reasoning, tool calls, sources, and more.

topcoat-prompt is a community project, not part of Topcoat or maintained by tokio-rs.

topcoat-prompt is a `topcoat ui` component registry. Components are copied into your app as
source you own, like the built-in components, and use the built-in neutral theme's tokens,
so they match the rest of your UI in light and dark mode. They are presentational: your
app owns the conversation state and talks to the model.

## Install

Your app needs Topcoat's `ui`, `tailwind`, and `icon-iconify` features, a theme from
`topcoat ui init`, and Lucide icons staged in `build.rs` (the components use
`iconify_icon!("lucide:…")`).

```sh
cargo add topcoat --features ui,tailwind,icon-iconify
cargo add topcoat-prompt
topcoat ui init --theme neutral
topcoat ui add message conversation prompt_input --registry topcoat-prompt
```

`topcoat ui` only offers registries that are direct dependencies of your app, which is
why `topcoat-prompt` is in `[dependencies]`. The crate contains no code, so it adds nothing
to your binary.

Built-in components a topcoat-prompt component uses (button, tooltip, avatar, badge, …) are
installed from the built-in registry automatically. To install everything:

```sh
topcoat ui add --all --registry topcoat-prompt
```

The `markdown` component renders with pulldown-cmark, which your app adds itself:

```sh
cargo add pulldown-cmark --no-default-features --features html
```

Stage Lucide in `build.rs`:

```rust
fn main() {
    topcoat::tailwind::BuildConfig::new().input("styles.css").render().unwrap();
    topcoat::icon::iconify::BuildConfig::new().icon_set("lucide").stage().unwrap();
}
```

## Components

| Name | What it is |
| --- | --- |
| `conversation` | A `role="log"` scroller that follows new content while you are at the bottom, stops following when you scroll up, and shows a "scroll to latest" button. Empty state included. Pure CSS (`flex-col-reverse`), so it works before the runtime loads. |
| `message` | A message row by role (user, assistant, system), with an avatar, content as a bubble or flat, and an actions toolbar shown on hover or focus. |
| `prompt_input` | A form with an auto-growing text area (Enter sends, Shift+Enter adds a line), a toolbar, tool buttons, and a submit button that becomes Stop while a response is on its way. |
| `markdown` | Server-side markdown with GFM tables, task lists and strikethrough. Raw HTML is shown as text and only `http`, `https` and `mailto` links are kept, so model output is safe to render. Tolerates half-streamed input. |
| `code_block` | A code block with its language and a copy button, highlighted on the server for Rust, Python, JavaScript and TypeScript (with JSX), JSON, YAML, TOML, shell, HTML, CSS, SQL, Go, C-family languages and diffs, with no extra crate. Used by `markdown` for fenced code. |
| `copy_button` | An icon button that copies text and confirms with a check. |
| `reasoning` | A collapsible "Thinking…" section, open while it streams and collapsed to "Thought for N seconds" after. |
| `tool_call` | A collapsible card per tool call with a status badge (pending, running, completed, error), input, and output. |
| `sources` | A collapsible list of sources, numbered inline citations, and source chips with a favicon, all with hover cards. |
| `suggestion` | Prompt suggestions, for an empty conversation or follow-ups, or as autocomplete rows with the typed text highlighted. |
| `loader` | Indicators for the wait before the first token: dots, typing, pulse, a spinner, wave, bars, pulsing text or shimmering text, in three sizes. |
| `attachment` | File chips with an image thumbnail or file-type icon and a remove button. |
| `branch_picker` | Previous / next and "2 / 3" for switching between regenerated responses. |
| `text_shimmer` | Text with a highlight sweeping across it, for a streaming status. |
| `thinking_bar` | A "Thinking" status line with an optional button to stop and answer now. |
| `steps` | A collapsible list of what an agent did, with a bar down the side. |
| `chain_of_thought` | Collapsible reasoning steps joined by a connector line. |
| `feedback_bar` | "Was this useful?" with helpful, not helpful and dismiss buttons that carry values. |
| `system_message` | An info, warning or error notice inside the conversation, optionally filled, with a call to action. |
| `file_upload` | A drop zone around any content backed by a real file input, with a picker button and an overlay while dragging. |
| `image` | An image from a URL or base64 data with a skeleton while it loads. |

Each component's doc comment has a usage example. A small chat looks like this:

```rust
view! {
    conversation(
        conversation_content(
            for entry in messages {
                message(role: entry.role, message_content(markdown(source: &entry.text)))
            }
        )
    )
    prompt_input(
        attrs: attributes! {
            @submit=$(async |e: Event| {
                e.prevent_default();
                let accepted = send(draft.get()).await;
                if accepted {
                    draft.set("".to_owned());
                }
            })
        },
        prompt_input_textarea(
            attrs: attributes! {
                :value=$(draft.get())
                @input=$(|e: Event| draft.set(e.target.value))
            }
        )
        prompt_input_toolbar(
            prompt_input_tools()
            prompt_input_submit(status: status, disabled: $(draft.get().trim().is_empty()))
        )
    )
}
```

## Docs site

`site/` is the documentation website: an introduction, installation, a page per
component (live examples with their code, install instructions with the full source, and
API tables), ten larger blocks, and `/llms.txt`. It installs every component.

```sh
cd site
topcoat dev
```

The site has one source of truth for each thing it shows. Every example is a real
`#[component]` in `site/src/examples/`, rendered live and shown with `include_str!`. The
manual install source is the registry file itself, and the API tables are read from the
component signatures and doc comments. `site/src/catalog.rs` lists the pages and drives
the sidebar, previous/next links and `llms.txt`; a test fails if a registry component has
no page or no example, and another if a documented prop or variant has no example. Code
panels go through the registry's own `code_block`, so the site uses the same highlighter
and colors as an app that installs it.

The **Full chat app** block (`/blocks`, and alone at `/full/chat-app`) is a working chat
answered by a fake model (no API keys, no network). A Tokio task streams reasoning, a
tool call that runs and completes, then markdown with a code block and sources. The live
region only observes the conversation, so reloading the page picks the stream up where it
is. Stop cancels the task; Regenerate adds a branch.

## Development

Component sources live in `crates/topcoat-prompt/src/components` and are listed in
`crates/topcoat-prompt/registry.toml`. The registry crate does not compile them; the site
does. After changing a component, copy it into the site:

```sh
cd site
topcoat ui add <name> --registry topcoat-prompt --overwrite
```

`cargo test --workspace` fails if an installed copy differs from the registry source, a
registry component is not installed, or a prop has no doc comment. The component files
are not part of the registry crate's module tree, so `cargo fmt` does not reach them;
format them with `rustfmt --edition 2024 crates/topcoat-prompt/src/components/*.rs` and
`topcoat fmt`.

## Releasing

1. Bump `version` in the root `Cargo.toml` in a pull request and merge it.
2. Tag the merge commit on `main` with that version and push the tag:

   ```sh
   git tag v0.2.1 && git push origin v0.2.1
   ```

The release workflow (`.github/workflows/release.yml`) checks that the tag matches
`Cargo.toml` and is on `main`, then publishes `topcoat-prompt` through crates.io trusted
publishing, so no crates.io token is stored in the repository.

## License

MIT
