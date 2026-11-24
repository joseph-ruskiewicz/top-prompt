use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, page},
    view::{View, view},
};

use crate::{
    catalog::{self, ComponentDoc},
    shell::{
        api_section, code_panel, command, example_section, install_section, page_header, pager,
        prose, section_heading,
    },
};

module_param!(pub slug: String else not_found);

/// `/docs/<slug>`: the Introduction, Installation, or a component's page.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let slug = slug(cx)?;
    let path = format!("/docs/{slug}");
    let doc = match slug.as_str() {
        "introduction" | "installation" => None,
        name => Some(catalog::component(name).ok_or_not_found()?),
    };

    Ok(view! {
        match doc {
            Some(doc) => component_page(doc: doc),
            None => {
                if slug == "introduction" {
                    introduction()
                } else {
                    installation()
                }
            }
        }
        pager(path: &path)
    })
}

/// A component's page: examples, installation and API.
#[topcoat::view::component]
async fn component_page(cx: &Cx, doc: &'static ComponentDoc) -> Result<impl View> {
    Ok(view! {
        page_header(title: doc.title, description: doc.description)
        section_heading(id: "examples", "Examples")
        <div class="flex flex-col">
            #[key(example.id)]
            for example in doc.examples {
                example_section(example: example)
            }
        </div>
        install_section(doc: doc)
        api_section(doc: doc)
    })
}

#[topcoat::view::component]
async fn introduction(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        page_header(
            title: "Introduction",
            description: "Components for chat interfaces in Topcoat apps."
        )
        prose(
            <p>
                "topcoat-prompt is a set of components for building chat interfaces with "
                <a href="https://github.com/tokio-rs/topcoat">"Topcoat"</a>
                ": conversations that follow streaming replies, messages, a prompt input, \
                markdown, reasoning, tool calls, sources and more."
            </p>
            <p>
                "It is not a library you depend on. Like Topcoat's built-in components, each \
                one is a Rust source file that "
                <code>"topcoat ui add"</code>
                " copies into your app. From then on it is your code: read it, change it, \
                delete what you don't need."
            </p>
            <h2>"How it fits together"</h2>
            <ul>
                <li>
                    "Components render on the server, as HTML. Pages work before any \
                    script loads, and the conversation sticks to the bottom with CSS alone."
                </li>
                <li>
                    "Interactive parts, such as copying code, submitting on Enter or keeping \
                    a section open while a reply streams, use the Topcoat runtime. There is \
                    no JavaScript to write and no WebAssembly."
                </li>
                <li>
                    "Streaming comes from Topcoat's live regions: your server renders the \
                    conversation again as tokens arrive, and the browser patches the page in \
                    place."
                </li>
                <li>
                    "Everything uses the tokens of the theme "
                    <code>"topcoat ui init"</code>
                    " installs, so the components match the rest of your app in light and \
                    dark mode."
                </li>
            </ul>
            <h2>"What it is not"</h2>
            <p>
                "topcoat-prompt does not talk to a model. The components are presentational: \
                your app owns the conversation, calls whichever model it likes, and renders \
                the result with these components. The "
                <a href="/blocks#chat_app">"full chat app"</a>
                " block shows one way to wire it up, with a fake model."
            </p>
        )
    })
}

/// The `build.rs` the Installation page shows.
const BUILD_RS: &str = r#"fn main() {
    // The theme stylesheet is the Tailwind input.
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();

    // The components use Lucide icons through `iconify_icon!`.
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("lucide")
        .stage()
        .unwrap();
}
"#;

/// The dependency sections the Installation page shows.
const CARGO_TOML: &str = r#"[dependencies]
topcoat = { version = "0.12", features = ["icon-iconify", "tailwind", "ui"] }
topcoat-prompt = "0.1"

[build-dependencies]
topcoat = { version = "0.12", default-features = false, features = ["icon-iconify", "tailwind"] }
"#;

#[topcoat::view::component]
async fn installation(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        page_header(
            title: "Installation",
            description: "Add the registry, set up a theme, and install components."
        )
        prose(
            <h2>"1. Add the crates"</h2>
            <p>
                "Your app needs Topcoat with the "
                <code>"ui"</code>
                ", "
                <code>"tailwind"</code>
                " and "
                <code>"icon-iconify"</code>
                " features, and the "
                <code>"topcoat-prompt"</code>
                " crate. The crate has no code of its own: it carries the component sources, \
                and "
                <code>"topcoat ui"</code>
                " only offers registries your app depends on directly."
            </p>
            code_panel(source: CARGO_TOML, language: Some("toml"))
            <h2>"2. Install a theme"</h2>
            <p>
                "The components use the design tokens of a Topcoat theme. If your app does \
                not have one yet, install the neutral theme; it writes "
                <code>"styles.css"</code>
                " and "
                <code>"components.toml"</code>
                "."
            </p>
            command(text: "topcoat ui init --theme neutral")
            <h2>"3. Stage the icons"</h2>
            <p>
                "The components draw Lucide icons. Stage the set in "
                <code>"build.rs"</code>
                ", next to the Tailwind build:"
            </p>
            code_panel(source: BUILD_RS)
            <h2>"4. Add components"</h2>
            <p>
                "Install components by name. Built-in Topcoat components they use, such as "
                <code>"button"</code>
                " and "
                <code>"tooltip"</code>
                ", are installed with them."
            </p>
            command(
                text: "topcoat ui add prompt_input conversation message --registry topcoat-prompt"
            )
            <p>"Or install everything at once:"</p>
            command(text: "topcoat ui add --all --registry topcoat-prompt")
            <p>
                "The sources land in "
                <code>"src/components/"</code>
                " and are declared in "
                <code>"src/components.rs"</code>
                ". Import them from there:"
            </p>
            code_panel(
                source: "use crate::components::prompt_input::{prompt_input, prompt_input_textarea};\n"
            )
            <h2>"Extra crates"</h2>
            <p>"A few components need a crate besides Topcoat:"</p>
            <ul>
                for doc in catalog::COMPONENTS
                    .iter()
                    .filter(|doc| !doc.crates.is_empty()) {
                    <li>
                        <a href=(format!("/docs/{}", doc.name))>(doc.title)</a>
                        ": "
                        for krate in doc.crates {
                            <code>(format!("cargo add {krate}"))</code>
                        }
                    </li>
                }
            </ul>
        )
    })
}
