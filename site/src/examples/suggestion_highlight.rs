use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::suggestion::{suggestion, suggestions};

/// Prompts that start with what the reader typed, as an app might look them up.
const MATCHES: [&str; 4] = [
    "How do I write a unit test in Rust?",
    "How do I read a file line by line?",
    "How do I parse JSON with serde?",
    "How do I spawn a background task?",
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let typed = "how do i";

    Ok(view! {
        <div
            class="flex w-full max-w-md flex-col gap-1 rounded-xl border border-border p-1.5"
        >
            <p class="px-3 pt-1.5 pb-1 text-xs text-muted-foreground">
                "Suggestions for “"
                (typed)
                "”"
            </p>
            suggestions(
                attrs: attributes! { class="flex-col gap-0" },
                for prompt in MATCHES {
                    suggestion(prompt: prompt, highlight: Some(typed))
                }
            )
        </div>
    })
}
