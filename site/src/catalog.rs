//! What the docs say about each component, in one place.
//!
//! `COMPONENTS` drives the sidebar, the component pages, their previous and next links,
//! and `llms.txt`. Each example is a real component in `src/examples/`: the page renders
//! it as the preview and shows the same file, through `include_str!`, as its code. The
//! manual-install source and the API tables come from the registry crate's files. So
//! the only prose here is a title, a one-line description and the example headings.

use topcoat::{
    context::Cx,
    view::{BoxView, ViewExt, view},
};

/// One component's docs page.
pub struct ComponentDoc {
    /// The registry name, used in `topcoat ui add` and in the page URL.
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Whether the component is new in this release, which the sidebar marks.
    pub new: bool,
    /// Crates the component needs besides Topcoat, as `cargo add` arguments.
    pub crates: &'static [&'static str],
    /// The registry source of the component.
    pub source: &'static str,
    pub examples: &'static [Example],
}

/// An example: a component in `src/examples/` and its source.
pub struct Example {
    /// The file name without `.rs`, unique across the site.
    pub id: &'static str,
    pub title: &'static str,
    /// A sentence under the title, or `""`.
    pub description: &'static str,
    /// The example's source file.
    pub source: &'static str,
    /// Renders the example.
    pub render: fn(&Cx) -> BoxView<'_>,
}

/// Declares an example from its file in `src/examples/`.
macro_rules! example {
    ($file:ident, $title:literal $(, $description:literal)? $(,)?) => {{
        fn render(cx: &Cx) -> BoxView<'_> {
            view! { crate::examples::$file::example() }.boxed()
        }
        Example {
            id: stringify!($file),
            title: $title,
            description: example!(@description $($description)?),
            source: include_str!(concat!("examples/", stringify!($file), ".rs")),
            render,
        }
    }};
    (@description $description:literal) => { $description };
    (@description) => { "" };
}

/// The registry source of the component `name`.
macro_rules! registry_source {
    ($name:literal) => {
        include_str!(concat!(
            "../../crates/topcoat-prompt/src/components/",
            $name,
            ".rs"
        ))
    };
}

/// Every component, in sidebar order: alphabetical by title.
pub static COMPONENTS: &[ComponentDoc] = &[
    ComponentDoc {
        name: "attachment",
        title: "Attachment",
        description: "File chips for files queued in a prompt or sent with a message.",
        new: false,
        crates: &[],
        source: registry_source!("attachment"),
        examples: &[
            example!(
                attachment_basic,
                "Attachments",
                "Give each chip a `name` and `media_type`; images with a `preview_src` show a thumbnail, and long names are truncated with the full name in a tooltip. A `removable` chip hides itself when its x is clicked."
            ),
            example!(
                attachment_app_state,
                "Removing files the app owns",
                "The chip only hides itself, so tell your app the file is gone through `remove_attrs`. Here the click calls a procedure that deletes the upload and returns the files left, and the composer re-renders with the new list and count. Key the chips in the loop."
            ),
            example!(
                attachment_custom,
                "A chip of your own",
                "`attachment_remove` is the chip's remove button on its own, for chips that aren't files, such as a pasted link. Its `name` goes into the button's label."
            ),
        ],
    },
    ComponentDoc {
        name: "branch_picker",
        title: "Branch Picker",
        description: "Previous and next controls for moving between regenerated responses.",
        new: false,
        crates: &[],
        source: registry_source!("branch_picker"),
        examples: &[
            example!(
                branch_picker_basic,
                "Paging between versions",
                "Your app keeps the versions and which one is shown; the picker renders the count and the buttons. Wire the buttons through their `attrs`, and disable them at either end. The copy button copies the version on screen."
            ),
            example!(
                branch_picker_regenerate,
                "Regenerating adds a version",
                "Regenerate asks the server for another answer, adds it to the list and shows it. The picker appears once there are two versions; `label` names it for screen readers."
            ),
        ],
    },
    ComponentDoc {
        name: "chain_of_thought",
        title: "Chain of Thought",
        description: "A model's reasoning as steps that open one at a time.",
        new: true,
        crates: &[],
        source: registry_source!("chain_of_thought"),
        examples: &[
            example!(
                chain_of_thought_basic,
                "Chain of thought",
                "Each step is a trigger and its content; `default_open` opens a step at first. The line joins each step to the next."
            ),
            example!(
                chain_of_thought_icons,
                "Step markers",
                "`leading_icon` replaces a step's dot, for example with what kind of work it was."
            ),
            example!(
                chain_of_thought_run,
                "While the model thinks",
                "Render the steps that have arrived and re-render as more come in, as a live region would. The step in progress opens and its title shimmers. Steps in a loop need a key, so each keeps its own open state."
            ),
        ],
    },
    ComponentDoc {
        name: "code_block",
        title: "Code Block",
        description: "A bordered block of code with its language and a copy button.",
        new: false,
        crates: &[],
        source: registry_source!("code_block"),
        examples: &[
            example!(
                code_block_basic,
                "Code block",
                "`code` is highlighted for its `language`, which also labels the header. The copy button copies `code`."
            ),
            example!(
                code_block_languages,
                "Languages",
                "Rust, Python, JavaScript and TypeScript with JSX, JSON, YAML, TOML, shell, HTML, CSS, SQL, Go, C-family and diffs are highlighted on the server, with common aliases such as `py`, `ts`, `sh` and `yml`."
            ),
            example!(
                code_block_plain,
                "Plain text",
                "Without `language`, or with one the highlighter does not know, the code is plain text and the header reads \"Code\" or the language. Long lines scroll instead of wrapping, and classes in `attrs` can cap the height."
            ),
            example!(
                code_block_opt_out,
                "Turning highlighting off",
                "`highlight: false` shows plain text even for a supported language."
            ),
            example!(
                code_block_markup,
                "Your own markup",
                "Child content replaces the text inside `<code>`, for emphasis of your own or another highlighter. `code` stays the text the button copies."
            ),
            example!(
                code_block_custom,
                "Your own frame",
                "`code_block_header` and `code_block_copy` also work on their own. `highlight` returns the escaped markup and `TOKEN_COLORS` colors it, for a block with more actions, such as a download link."
            ),
        ],
    },
    ComponentDoc {
        name: "conversation",
        title: "Conversation",
        description: "A scrolling log that follows new messages while you are at the bottom.",
        new: false,
        crates: &[],
        source: registry_source!("conversation"),
        examples: &[
            example!(
                conversation_basic,
                "Conversation",
                "Put messages in a `conversation_content` inside a `conversation` that has a height to fill."
            ),
            example!(
                conversation_empty,
                "Empty state",
                "`conversation_empty_state` takes a `title`, a `description` and any extra content, such as an icon or suggestions."
            ),
            example!(
                conversation_streaming,
                "Following a streaming reply",
                "While the reader is at the bottom, new words keep them there; scroll up during the reply and the log stops following and offers a button back down. Set `aria-busy` through `attrs` while streaming."
            ),
            example!(
                conversation_custom_log,
                "A log of your own",
                "`conversation` renders its scroll button itself. For another scrolling log, give `conversation_scroll_button` the ID of a reversed column as `target`, and `visible` while the reader is scrolled up."
            ),
        ],
    },
    ComponentDoc {
        name: "copy_button",
        title: "Copy Button",
        description: "An icon button that copies text and confirms with a check.",
        new: false,
        crates: &[],
        source: registry_source!("copy_button"),
        examples: &[
            example!(
                copy_button_basic,
                "Copy button",
                "`text` is what goes on the clipboard and `label` names the button. The icon turns into a check for two seconds."
            ),
            example!(
                copy_button_generated,
                "Copying something new",
                "`text` is fixed when the button renders, so render it again when the value changes. Here the server makes a new link and the button copies that one."
            ),
        ],
    },
    ComponentDoc {
        name: "feedback_bar",
        title: "Feedback Bar",
        description: "Asks whether a response helped, with thumbs up, thumbs down and close.",
        new: true,
        crates: &[],
        source: registry_source!("feedback_bar"),
        examples: &[
            example!(
                feedback_bar_basic,
                "Feedback bar",
                "Each button carries a value, `helpful`, `not_helpful` or `close`, so your handler knows which was pressed. Here the choice is acknowledged in the browser."
            ),
            example!(
                feedback_bar_server,
                "Saving the rating",
                "Send the value to a procedure and show what it returns. `title` and each button's `label` change the wording."
            ),
        ],
    },
    ComponentDoc {
        name: "file_upload",
        title: "File Upload",
        description: "A drop zone around a real file input, with a picker button and overlay.",
        new: true,
        crates: &[],
        source: registry_source!("file_upload"),
        examples: &[
            example!(
                file_upload_basic,
                "Drop zone around a prompt",
                "Wrap any content: dropped files go into the real file input, which fires `change` as if they had been picked. The trigger opens the picker, and `file_upload_content` shows while files are over the zone."
            ),
            example!(
                file_upload_attachments,
                "Files as attachment chips",
                "Copy the input's files into a signal on `change` and render a chip for each. Removing a chip also takes the file out of the input, so the form sends only what is shown."
            ),
            example!(
                file_upload_single,
                "One image",
                "`accept` limits the picker and `multiple` off keeps one file; `name` is the form field. The trigger takes the button's `variant` and `size`."
            ),
        ],
    },
    ComponentDoc {
        name: "image",
        title: "Image",
        description: "A generated or attached image, from a URL or base64 data.",
        new: true,
        crates: &[],
        source: registry_source!("image"),
        examples: &[
            example!(
                image_basic,
                "Image",
                "Pass a `src` URL and an `alt` that describes the image."
            ),
            example!(
                image_base64,
                "Inline image data",
                "For images a model returns as data, pass `base64` and its `media_type`. `width` and `height` give the placeholder its shape."
            ),
            example!(
                image_generated,
                "A generated image in a reply",
                "A placeholder pulses until the image loads. Here the reply shows a thinking bar while the server makes the image, then the image."
            ),
        ],
    },
    ComponentDoc {
        name: "loader",
        title: "Loader",
        description: "Indicators for the wait before the first token.",
        new: false,
        crates: &[],
        source: registry_source!("loader"),
        examples: &[
            example!(
                loader_variants,
                "Variants",
                "`variant` picks the animation."
            ),
            example!(
                loader_sizes,
                "Sizes",
                "`size` scales it: `Sm` next to small text, `Md` in a message, `Lg` in an empty panel."
            ),
            example!(
                loader_waiting,
                "Until the first token",
                "Show a loader in the message until text arrives. `label` describes the wait to screen readers, and the Text variants show it."
            ),
        ],
    },
    ComponentDoc {
        name: "markdown",
        title: "Markdown",
        description: "Server-rendered markdown that is safe for model output.",
        new: false,
        crates: &["pulldown-cmark --no-default-features --features html"],
        source: registry_source!("markdown"),
        examples: &[
            example!(
                markdown_basic,
                "Markdown",
                "Headings, lists, tables, task lists, links and fenced code, rendered on the server."
            ),
            example!(
                markdown_code,
                "Code",
                "Fenced code renders as a `code_block`, highlighted for the fence's language. An unknown or missing language shows plain text."
            ),
            example!(
                markdown_streaming,
                "Streaming",
                "Render the text received so far on every update. Half a code fence or half a link renders as far as it goes, and the code is highlighted as it arrives."
            ),
            example!(
                markdown_untrusted,
                "Untrusted input",
                "Model output is safe to pass as it is: HTML stays text, and only http, https and mailto links survive. `attrs` add classes to the wrapper."
            ),
        ],
    },
    ComponentDoc {
        name: "message",
        title: "Message",
        description: "A message by role, with an avatar, a bubble or flat body, and actions.",
        new: false,
        crates: &[],
        source: registry_source!("message"),
        examples: &[
            example!(
                message_basic,
                "Messages",
                "`role` sets the layout: users on the right, the assistant full width, system notes centered."
            ),
            example!(
                message_variants,
                "Content variants",
                "`Auto`, the default, picks a bubble for users and flat text for the rest. `Bubble` and `Flat` force one look."
            ),
            example!(
                message_avatars,
                "Avatars",
                "`src` shows an image over the `fallback` initials, which stay if the image can't load. Give `alt` text when the message doesn't name its author otherwise."
            ),
            example!(
                message_actions,
                "Actions",
                "Add `message_action` buttons, a copy button or a branch picker. Each action's `label` is its tooltip and name; `side` moves the tooltip. Wire clicks through `attrs`."
            ),
        ],
    },
    ComponentDoc {
        name: "prompt_input",
        title: "Prompt Input",
        description: "A composer with a growing text area, tool buttons and a send button.",
        new: false,
        crates: &[],
        source: registry_source!("prompt_input"),
        examples: &[
            example!(
                prompt_input_basic,
                "Prompt input",
                "Keep the text in a signal and handle `@submit` on the form. Enter submits and Shift+Enter adds a line."
            ),
            example!(
                prompt_input_actions,
                "Tools",
                "Put `prompt_input_button`s in `prompt_input_tools`: icon buttons need an `aria-label`, and `size: Sm` fits a text label. Send something to see the button turn into Stop."
            ),
            example!(
                prompt_input_status,
                "Request status",
                "`status` follows the request: Ready sends, Submitted and Streaming turn the button into Stop, and Error turns it into Retry. Handle Stop with `@click` in `attrs`. Tick the box to see a failure."
            ),
        ],
    },
    ComponentDoc {
        name: "reasoning",
        title: "Reasoning",
        description: "Collapsible reasoning: open while it streams, then summarized.",
        new: false,
        crates: &[],
        source: registry_source!("reasoning"),
        examples: &[
            example!(
                reasoning_basic,
                "Reasoning",
                "While `streaming`, the section is open and shimmers. Afterwards it is closed and shows `duration_secs`."
            ),
            example!(
                reasoning_run,
                "While it streams",
                "Re-render as thoughts arrive with `streaming: true`, then once more with `false`: the section closes itself. If the reader opens or closes it meanwhile, that choice is kept."
            ),
            example!(
                reasoning_open,
                "Open after it finishes",
                "`reasoning` closes once `streaming` ends. To keep it open, put `reasoning_trigger` and the content in a `<details>` of your own."
            ),
        ],
    },
    ComponentDoc {
        name: "sources",
        title: "Sources",
        description: "Sources as a collapsible list, compact chips, or inline citations.",
        new: false,
        crates: &[],
        source: registry_source!("sources"),
        examples: &[
            example!(
                sources_list,
                "Source list",
                "`sources` shows \"Used N sources\" from `count` and opens to the links. Each `source` shows its `title`, or its URL without one."
            ),
            example!(
                sources_chips,
                "Source chips",
                "Hover or focus a chip for its title and description. Without a `label`, a chip shows the host; `show_favicon` adds the site's icon."
            ),
            example!(
                sources_citations,
                "Inline citations",
                "Numbered markers after a claim, with a hover card naming the source."
            ),
            example!(
                sources_answer,
                "Under an answer",
                "How the parts sit together in an assistant message: citations in the text, chips under it, and the full list last."
            ),
            example!(
                sources_open,
                "Open from the start",
                "For a list that starts open, put `sources_trigger` and the content in a `<details>` of your own."
            ),
        ],
    },
    ComponentDoc {
        name: "steps",
        title: "Steps",
        description: "A collapsible list of what an agent did, with a bar down the side.",
        new: true,
        crates: &[],
        source: registry_source!("steps"),
        examples: &[
            example!(
                steps_basic,
                "Steps",
                "A trigger with an optional `leading_icon`, and items beside a bar. `default_open` opens the list at first."
            ),
            example!(
                steps_run,
                "An agent at work",
                "Re-render as the agent reports each operation. The icon and shimmering title show it is still running."
            ),
            example!(
                steps_flat,
                "Without collapsing",
                "`steps_bar` and `steps_item` work without the collapsible, for a list that is always visible."
            ),
        ],
    },
    ComponentDoc {
        name: "suggestion",
        title: "Suggestion",
        description: "Prompt suggestions as buttons, or as an autocomplete list.",
        new: false,
        crates: &[],
        source: registry_source!("suggestion"),
        examples: &[
            example!(
                suggestion_basic,
                "Suggestions",
                "Each suggestion shows its `prompt`, or a shorter `label`."
            ),
            example!(
                suggestion_fill,
                "Filling the prompt input",
                "A click handler reads the prompt from the button's value and puts it in the composer."
            ),
            example!(
                suggestion_submit,
                "Sending without JavaScript",
                "As submit buttons with a `name` inside a form, suggestions send their prompt with the form."
            ),
            example!(
                suggestion_highlight,
                "Autocomplete",
                "With `highlight`, a suggestion becomes a full-width row with the typed text emphasized. Here the server filters as you type, and picking a row fills the input."
            ),
        ],
    },
    ComponentDoc {
        name: "system_message",
        title: "System Message",
        description: "A notice inside the conversation: information, a warning, or an error.",
        new: true,
        crates: &[],
        source: registry_source!("system_message"),
        examples: &[
            example!(
                system_message_variants,
                "Variants",
                "`variant` sets the tone and icon: Action, Warning or Error. `fill` tints the background and `leading_icon` swaps the icon. A `system_message_cta` with an `href` is a link."
            ),
            example!(
                system_message_retry,
                "Acting on a notice",
                "The action is a button you wire through `attrs`. Here Retry calls the server and the notice changes with the result."
            ),
        ],
    },
    ComponentDoc {
        name: "text_shimmer",
        title: "Text Shimmer",
        description: "Muted text with a highlight sweeping across it.",
        new: true,
        crates: &[],
        source: registry_source!("text_shimmer"),
        examples: &[
            example!(
                text_shimmer_basic,
                "Text shimmer",
                "`duration_ms` sets the speed of the sweep and `spread` its width. Size and weight come from classes in `attrs`."
            ),
            example!(
                text_shimmer_status,
                "A changing status",
                "Swap the text as the work moves on. Put it in a `role=\"status\"` element so each change is announced."
            ),
        ],
    },
    ComponentDoc {
        name: "thinking_bar",
        title: "Thinking Bar",
        description: "A shimmering \"Thinking\" row with an optional button to answer now.",
        new: true,
        crates: &[],
        source: registry_source!("thinking_bar"),
        examples: &[
            example!(
                thinking_bar_basic,
                "Thinking bar",
                "`text` shimmers. With a `stop_label`, a button follows it; wire it in `stop_attrs`."
            ),
            example!(
                thinking_bar_stop,
                "Answering early",
                "Stop tells the server to answer with what the model has. Here the short answer replaces the bar."
            ),
        ],
    },
    ComponentDoc {
        name: "tool_call",
        title: "Tool Call",
        description: "A collapsible card per tool call, with its status, input and output.",
        new: false,
        crates: &[],
        source: registry_source!("tool_call"),
        examples: &[
            example!(
                tool_call_statuses,
                "Statuses",
                "`status` sets the badge. `tool_call_output` with `error: true` labels a failure."
            ),
            example!(
                tool_call_run,
                "A call in progress",
                "Render the card again with each new status. If the reader opened it, it stays open. `tool_call_input` takes a `label` for its heading."
            ),
            example!(
                tool_call_open,
                "Open from the start",
                "For a card that starts open, build it around `tool_call_header`, which shows the name and status badge."
            ),
        ],
    },
];

/// The example on the home page.
pub static HOME_EXAMPLE: Example = example!(prompt_input_actions, "Prompt input");

/// A larger composed example on the Blocks page, with a full-page view.
pub struct Block {
    /// The URL segment of the full-page view, `/full/<slug>`.
    pub slug: &'static str,
    pub example: Example,
}

/// The blocks, in page order.
pub static BLOCKS: &[Block] = &[
    Block {
        slug: "prompt-input-actions",
        example: example!(block_prompt_input_actions, "Prompt input with actions"),
    },
    Block {
        slug: "prompt-input-suggestions",
        example: example!(
            block_prompt_input_suggestions,
            "Prompt input with suggestions"
        ),
    },
    Block {
        slug: "prompt-input-autocomplete",
        example: example!(
            block_prompt_input_autocomplete,
            "Prompt input with autocomplete",
            "Type to see matching prompts, with the typed text highlighted in each."
        ),
    },
    Block {
        slug: "conversation-basic",
        example: example!(block_conversation_basic, "Basic conversation"),
    },
    Block {
        slug: "conversation-avatars",
        example: example!(block_conversation_avatars, "Conversation with avatars"),
    },
    Block {
        slug: "conversation-actions",
        example: example!(block_conversation_actions, "Conversation with actions"),
    },
    Block {
        slug: "conversation-scroll",
        example: example!(
            block_conversation_scroll,
            "Conversation with scroll to bottom",
            "Scroll up to release the log from the bottom and show the button."
        ),
    },
    Block {
        slug: "conversation-prompt-input",
        example: example!(
            block_conversation_prompt_input,
            "Conversation with prompt input"
        ),
    },
    Block {
        slug: "sidebar-history",
        example: example!(block_sidebar_history, "Sidebar with chat history"),
    },
    Block {
        slug: "chat-app",
        example: example!(
            chat_app,
            "Full chat app",
            "A working chat answered by a fake model: a Tokio task streams reasoning, a tool \
             call and a markdown answer. The conversation is shared by everyone viewing it. \
             The real-model demo below runs the same components on the Claude API."
        ),
    },
];

/// The block with the full-page view `slug`.
pub fn block(slug: &str) -> Option<&'static Block> {
    BLOCKS.iter().find(|block| block.slug == slug)
}

/// A component another one needs installed.
#[derive(Debug, PartialEq, Eq)]
pub enum Dependency {
    /// A built-in Topcoat component.
    BuiltIn(String),
    /// Another topcoat-prompt component.
    Local(String),
}

/// The registry manifest, read for component dependencies.
const REGISTRY: &str = include_str!("../../crates/topcoat-prompt/registry.toml");

/// The dependencies `registry.toml` lists for the component `name`.
pub fn dependencies(name: &str) -> Vec<Dependency> {
    let header = format!("[components.{name}]");
    let Some(start) = REGISTRY.find(&header) else {
        return Vec::new();
    };
    let section = &REGISTRY[start + header.len()..];
    let section = &section[..section.find("\n[").unwrap_or(section.len())];
    let Some(list) = section
        .split_once("dependencies = [")
        .map(|(_, rest)| &rest[..rest.rfind(']').unwrap_or(rest.len())])
    else {
        return Vec::new();
    };

    let mut dependencies = Vec::new();
    let mut rest = list;
    while let Some(start) = rest.find(['"', '{']) {
        if rest[start..].starts_with('"') {
            let name = &rest[start + 1..];
            let end = name.find('"').unwrap_or(name.len());
            dependencies.push(Dependency::Local(name[..end].to_owned()));
            rest = &name[end + 1..];
        } else {
            let table = &rest[start..];
            let end = table.find('}').unwrap_or(table.len());
            let name = table[..end]
                .split_once("name = \"")
                .map(|(_, name)| &name[..name.find('"').unwrap_or(name.len())])
                .unwrap_or_default();
            dependencies.push(Dependency::BuiltIn(name.to_owned()));
            rest = &table[end..];
        }
    }
    dependencies
}

/// A page in sidebar order, for previous and next links.
pub struct Page {
    pub title: &'static str,
    pub href: String,
}

/// Every docs page in sidebar order.
pub fn pages() -> Vec<Page> {
    let page = |title, href: &str| Page {
        title,
        href: href.to_owned(),
    };
    let mut pages = vec![
        page("Home", "/"),
        page("Introduction", "/docs/introduction"),
        page("Installation", "/docs/installation"),
    ];
    pages.extend(
        COMPONENTS
            .iter()
            .map(|doc| page(doc.title, &format!("/docs/{}", doc.name))),
    );
    pages.push(page("Blocks", "/blocks"));
    pages
}

/// `/llms.txt`: an index of the docs for language models.
pub fn llms_txt() -> String {
    let mut out = String::from(
        "# topcoat-prompt\n\n\
         > Chat interface components for Topcoat, the Rust web framework. Components are \
         installed into an app as source with `topcoat ui add <name> --registry topcoat-prompt` \
         and render on the server; interactive parts use the Topcoat runtime.\n\n\
         ## Docs\n\n\
         - [Introduction](/docs/introduction): what topcoat-prompt is and how it works\n\
         - [Installation](/docs/installation): adding the registry and installing components\n\
         - [Blocks](/blocks): larger composed examples, including a full chat app\n\
         - [Real model demo](/full/claude): the chat app on the Claude API, with your own key\n\n\
         ## Components\n\n",
    );
    for doc in COMPONENTS {
        out.push_str(&format!(
            "- [{}](/docs/{}): {} Install: `topcoat ui add {} --registry topcoat-prompt`\n",
            doc.title, doc.name, doc.description, doc.name
        ));
    }
    out
}

/// The docs of the component `name`.
pub fn component(name: &str) -> Option<&'static ComponentDoc> {
    COMPONENTS.iter().find(|doc| doc.name == name)
}

/// Every registry source the site documents, by component name.
#[cfg(test)]
pub fn registry_sources() -> impl Iterator<Item = (&'static str, &'static str)> {
    COMPONENTS.iter().map(|doc| (doc.name, doc.source))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, path::Path};

    use topcoat_ui::Registry;

    use super::COMPONENTS;

    #[test]
    fn every_registry_component_has_a_page_with_an_example() {
        let registry =
            Registry::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/topcoat-prompt"))
                .expect("the workspace's topcoat-prompt registry loads");
        let registry: BTreeSet<&str> = registry.names().collect();
        let documented: BTreeSet<&str> = COMPONENTS.iter().map(|doc| doc.name).collect();

        assert_eq!(
            registry, documented,
            "every registry component needs a docs entry"
        );
        for doc in COMPONENTS {
            assert!(!doc.examples.is_empty(), "{} has no example", doc.name);
        }
    }

    /// Every public part, prop and enum variant on a component's API tables shows up in
    /// at least one of the page's examples.
    ///
    /// The check reads source text: a part counts as used when `name(` appears, a prop
    /// when `prop:` does, and a variant when `Enum::Variant` does. That can be fooled by a
    /// prop of the same name on another part, but it does not depend on formatting, and
    /// it fails as soon as a new prop or variant is added without an example. `attrs` is
    /// on every part, so it only has to appear once per page.
    #[test]
    fn examples_use_every_part_prop_and_variant() {
        let mut gaps = Vec::new();
        for doc in COMPONENTS {
            let examples: String = doc.examples.iter().map(|example| example.source).collect();
            for function in crate::api::functions(doc.source) {
                if !examples.contains(&format!("{}(", function.name)) {
                    gaps.push(format!(
                        "{}: `{}` is in no example",
                        doc.name, function.name
                    ));
                }
                for prop in function.props {
                    let ignored = matches!(prop.name.as_str(), "attrs" | "children");
                    if !ignored && !examples.contains(&format!("{}:", prop.name)) {
                        gaps.push(format!(
                            "{}: `{}` prop `{}`",
                            doc.name, function.name, prop.name
                        ));
                    }
                }
            }
            if !examples.contains("attrs:") {
                gaps.push(format!("{}: no example passes `attrs`", doc.name));
            }
            for (name, variants) in enums(doc.source) {
                for variant in variants {
                    if !examples.contains(&format!("{name}::{variant}")) {
                        gaps.push(format!(
                            "{}: `{name}::{variant}` is in no example",
                            doc.name
                        ));
                    }
                }
            }
        }
        assert!(
            gaps.is_empty(),
            "undemonstrated features:\n{}",
            gaps.join("\n")
        );
    }

    /// The public enums of a component source and their variants.
    fn enums(source: &str) -> Vec<(&str, Vec<&str>)> {
        source
            .split("\npub enum ")
            .skip(1)
            .filter_map(|rest| {
                let (name, body) = rest.split_once(" {")?;
                let body = &body[..body.find("\n}")?];
                let variants = body
                    .lines()
                    .map(str::trim)
                    .filter(|line| {
                        !line.is_empty() && !line.starts_with("///") && !line.starts_with('#')
                    })
                    .map(|line| line.trim_end_matches(','))
                    .collect();
                Some((name, variants))
            })
            .collect()
    }

    #[test]
    fn reads_dependencies_from_the_registry() {
        use super::{Dependency, dependencies};

        assert_eq!(
            dependencies("thinking_bar"),
            [
                Dependency::Local("text_shimmer".to_owned()),
                Dependency::BuiltIn("button".to_owned()),
            ]
        );
        assert_eq!(dependencies("image"), []);
        assert_eq!(
            dependencies("message").len(),
            4,
            "message uses avatar, button, floating and tooltip"
        );
    }

    #[test]
    fn components_are_in_sidebar_order() {
        let titles: Vec<&str> = COMPONENTS.iter().map(|doc| doc.title).collect();
        let mut sorted = titles.clone();
        sorted.sort_unstable();
        assert_eq!(titles, sorted);
    }

    #[test]
    fn example_ids_are_unique() {
        let ids: Vec<&str> = COMPONENTS
            .iter()
            .flat_map(|doc| doc.examples)
            .chain(super::BLOCKS.iter().map(|block| &block.example))
            .map(|example| example.id)
            .collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }
}
