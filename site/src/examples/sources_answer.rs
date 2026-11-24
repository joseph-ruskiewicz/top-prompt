use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::{
    markdown::markdown,
    message::{message, message_content},
    sources::{inline_citation, source, source_chip, sources, sources_content},
};

const ANSWER: &str = "Sow tomatoes indoors **six to eight weeks** before the last frost, \
    and move them outside once nights stay above 10°C.";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="w-full">
            message(
                message_content(
                    markdown(source: ANSWER)
                    <p class="text-sm">
                        "Harden them off over a week first"
                        inline_citation(
                            label: "1",
                            href: "https://www.rhs.org.uk/advice/",
                            title: "Hardening off plants"
                        )
                        "."
                    </p>
                    // Chips for a quick look, with the host as their text by default.
                    <div class="flex flex-wrap gap-2">
                        source_chip(
                            href: "https://www.rhs.org.uk/advice/",
                            description: "Gardening advice from a horticultural charity.",
                            show_favicon: true
                        )
                        source_chip(
                            href: "https://extension.umn.edu/vegetables/growing-tomatoes",
                            label: "UMN Extension",
                            title: "Growing tomatoes in home gardens"
                        )
                    </div>
                    // And the full list, collapsed until the reader wants it.
                    sources(
                        count: 2,
                        // Classes in `attrs` go on the `<details>`.
                        attrs: attributes! { class="border-t border-border pt-3" },
                        sources_content(
                            source(
                                href: "https://www.rhs.org.uk/advice/",
                                title: "Hardening off plants"
                            )
                            source(
                                href: "https://extension.umn.edu/vegetables/growing-tomatoes"
                            )
                        )
                    )
                )
            )
        </div>
    })
}
