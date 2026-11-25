use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{Unescaped, View, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button_variants},
    code_block::{TOKEN_COLORS, code_block_copy, code_block_header, highlight},
};

const CONFIG: &str = "[server]
port = 8080
workers = 4

[database]
url = \"postgres://localhost/garden\"
";

/// The data URL a download link serves the file from.
const DOWNLOAD: &str = "data:text/plain;charset=utf-8,%5Bserver%5D%0Aport%20%3D%208080%0A\
    workers%20%3D%204%0A%0A%5Bdatabase%5D%0Aurl%20%3D%20%22postgres%3A%2F%2Flocalhost%2F\
    garden%22%0A";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // The header and copy button work outside `code_block`, for a block of your own
        // with more actions in the header.
        <figure
            class="w-full overflow-hidden rounded-xl border border-border bg-muted/40"
        >
            code_block_header(
                language: Some("toml"),
                <a
                    href=(DOWNLOAD)
                    download="garden.toml"
                    aria-label="Download garden.toml"
                    class=(button_variants(ButtonVariant::Ghost, ButtonSize::Icon))
                >
                    icon(data: iconify_icon!("lucide:download"))
                </a>
                code_block_copy(text: CONFIG)
            )
            <pre class="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
                // `highlight` returns escaped markup, and `TOKEN_COLORS` colors it.
                <code class=(TOKEN_COLORS)>
                    match highlight("toml", CONFIG) {
                        Some(html) => (Unescaped::new_unchecked(html)),
                        None => (CONFIG),
                    }
                </code>
            </pre>
            <figcaption
                class="border-t border-border px-4 py-2 text-xs text-muted-foreground"
            >
                "garden.toml, read at startup"
            </figcaption>
        </figure>
    })
}
