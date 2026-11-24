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
            "../../crates/top-prompt/src/components/",
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
        examples: &[example!(attachment_basic, "Attachments")],
    },
    ComponentDoc {
        name: "branch_picker",
        title: "Branch Picker",
        description: "Previous and next controls for moving between regenerated responses.",
        new: false,
        crates: &[],
        source: registry_source!("branch_picker"),
        examples: &[example!(
            branch_picker_basic,
            "Branch picker",
            "Put it in a message's actions row, next to the other actions."
        )],
    },
    ComponentDoc {
        name: "chain_of_thought",
        title: "Chain of Thought",
        description: "A model's reasoning as steps that open one at a time.",
        new: true,
        crates: &[],
        source: registry_source!("chain_of_thought"),
        examples: &[example!(chain_of_thought_basic, "Chain of thought")],
    },
    ComponentDoc {
        name: "code_block",
        title: "Code Block",
        description: "A bordered block of code with its language and a copy button.",
        new: false,
        crates: &[],
        source: registry_source!("code_block"),
        examples: &[
            example!(code_block_basic, "Code block"),
            example!(
                code_block_highlighted,
                "Highlighted",
                "The component does not highlight code. Highlight it in your app and pass \
                 the markup as children; the copy button still copies the plain text."
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
                "Scroll up inside the log: it stops following and offers a button back down."
            ),
            example!(conversation_empty, "Empty state"),
        ],
    },
    ComponentDoc {
        name: "copy_button",
        title: "Copy Button",
        description: "An icon button that copies text and confirms with a check.",
        new: false,
        crates: &[],
        source: registry_source!("copy_button"),
        examples: &[example!(copy_button_basic, "Copy button")],
    },
    ComponentDoc {
        name: "feedback_bar",
        title: "Feedback Bar",
        description: "Asks whether a response helped, with thumbs up, thumbs down and close.",
        new: true,
        crates: &[],
        source: registry_source!("feedback_bar"),
        examples: &[example!(
            feedback_bar_basic,
            "Feedback bar",
            "The buttons carry values; this example thanks the reader and then hides the bar."
        )],
    },
    ComponentDoc {
        name: "file_upload",
        title: "File Upload",
        description: "A drop zone around a real file input, with a picker button and overlay.",
        new: true,
        crates: &[],
        source: registry_source!("file_upload"),
        examples: &[example!(
            file_upload_basic,
            "File upload",
            "Drop files anywhere on the composer, or use the paperclip button."
        )],
    },
    ComponentDoc {
        name: "image",
        title: "Image",
        description: "A generated or attached image, from a URL or base64 data.",
        new: true,
        crates: &[],
        source: registry_source!("image"),
        examples: &[example!(image_basic, "Image")],
    },
    ComponentDoc {
        name: "loader",
        title: "Loader",
        description: "Indicators for the wait before the first token.",
        new: false,
        crates: &[],
        source: registry_source!("loader"),
        examples: &[
            example!(loader_variants, "Variants"),
            example!(loader_sizes, "Sizes"),
        ],
    },
    ComponentDoc {
        name: "markdown",
        title: "Markdown",
        description: "Server-rendered markdown that is safe for model output.",
        new: false,
        crates: &["pulldown-cmark --no-default-features --features html"],
        source: registry_source!("markdown"),
        examples: &[example!(
            markdown_basic,
            "Markdown",
            "Raw HTML in the source shows as text, and only http, https and mailto links \
             are kept."
        )],
    },
    ComponentDoc {
        name: "message",
        title: "Message",
        description: "A message by role, with an avatar, a bubble or flat body, and actions.",
        new: false,
        crates: &[],
        source: registry_source!("message"),
        examples: &[
            example!(message_basic, "Messages"),
            example!(
                message_actions,
                "With actions",
                "Actions show on hover and focus, and always on the last message."
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
            example!(prompt_input_basic, "Prompt input"),
            example!(
                prompt_input_actions,
                "With actions",
                "Tool buttons go on the left of the toolbar. Send something to see the button \
                 turn into Stop."
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
        examples: &[example!(reasoning_basic, "Reasoning")],
    },
    ComponentDoc {
        name: "sources",
        title: "Sources",
        description: "Sources as a collapsible list, compact chips, or inline citations.",
        new: false,
        crates: &[],
        source: registry_source!("sources"),
        examples: &[
            example!(sources_list, "Source list"),
            example!(
                sources_chips,
                "Source chips",
                "Hover or focus a chip for details."
            ),
            example!(sources_citations, "Inline citations"),
        ],
    },
    ComponentDoc {
        name: "steps",
        title: "Steps",
        description: "A collapsible list of what an agent did, with a bar down the side.",
        new: true,
        crates: &[],
        source: registry_source!("steps"),
        examples: &[example!(steps_basic, "Steps")],
    },
    ComponentDoc {
        name: "suggestion",
        title: "Suggestion",
        description: "Prompt suggestions as buttons, or as an autocomplete list.",
        new: false,
        crates: &[],
        source: registry_source!("suggestion"),
        examples: &[
            example!(suggestion_basic, "Suggestions"),
            example!(
                suggestion_highlight,
                "Highlight",
                "With `highlight`, suggestions are full-width rows that emphasize the typed \
                 text, for autocomplete lists."
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
        examples: &[example!(system_message_variants, "Variants")],
    },
    ComponentDoc {
        name: "text_shimmer",
        title: "Text Shimmer",
        description: "Muted text with a highlight sweeping across it.",
        new: true,
        crates: &[],
        source: registry_source!("text_shimmer"),
        examples: &[example!(text_shimmer_basic, "Text shimmer")],
    },
    ComponentDoc {
        name: "thinking_bar",
        title: "Thinking Bar",
        description: "A shimmering \"Thinking\" row with an optional button to answer now.",
        new: true,
        crates: &[],
        source: registry_source!("thinking_bar"),
        examples: &[example!(thinking_bar_basic, "Thinking bar")],
    },
    ComponentDoc {
        name: "tool_call",
        title: "Tool Call",
        description: "A collapsible card per tool call, with its status, input and output.",
        new: false,
        crates: &[],
        source: registry_source!("tool_call"),
        examples: &[example!(tool_call_statuses, "Statuses")],
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
             call and a markdown answer. The conversation is shared by everyone viewing it."
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
    /// Another top-prompt component.
    Local(String),
}

/// The registry manifest, read for component dependencies.
const REGISTRY: &str = include_str!("../../crates/top-prompt/registry.toml");

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
        "# top-prompt\n\n\
         > Chat interface components for Topcoat, the Rust web framework. Components are \
         installed into an app as source with `topcoat ui add <name> --registry top-prompt` \
         and render on the server; interactive parts use the Topcoat runtime.\n\n\
         ## Docs\n\n\
         - [Introduction](/docs/introduction): what top-prompt is and how it works\n\
         - [Installation](/docs/installation): adding the registry and installing components\n\
         - [Blocks](/blocks): larger composed examples, including a full chat app\n\n\
         ## Components\n\n",
    );
    for doc in COMPONENTS {
        out.push_str(&format!(
            "- [{}](/docs/{}): {} Install: `topcoat ui add {} --registry top-prompt`\n",
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
            Registry::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/top-prompt"))
                .expect("the workspace's top-prompt registry loads");
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
