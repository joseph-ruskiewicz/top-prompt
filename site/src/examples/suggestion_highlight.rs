use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::suggestion::{suggestion, suggestions};

/// Prompts the app can suggest. A real app might search a database instead.
const PROMPTS: [&str; 6] = [
    "How do I write a unit test in Rust?",
    "How do I read a file line by line?",
    "How do I parse JSON with serde?",
    "How do I spawn a background task?",
    "What is a lifetime?",
    "What does the question mark operator do?",
];

/// Re-renders on the server as the reader types, with the matches highlighted.
#[shard]
async fn matches(cx: &Cx, query: Signal<String>) -> Result<impl View> {
    let typed = query.get();
    let needle = typed.trim().to_lowercase();
    let found: Vec<&str> = PROMPTS
        .into_iter()
        .filter(|prompt| !needle.is_empty() && prompt.to_lowercase().contains(&needle))
        .collect();

    Ok(view! {
        if !found.is_empty() {
            suggestions(
                attrs: attributes! { class="flex-col gap-0" },
                #[key(prompt)]
                for prompt in found {
                    suggestion(
                        prompt: prompt,
                        highlight: Some(needle.as_str()),
                        // Picking a row fills the input with the whole prompt.
                        attrs: attributes! { @click=$(|e: Event| query.set(e.current_target.value)) }
                    )
                }
            )
        }
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || "how do".to_owned());
    let rows = query.clone();

    Ok(view! {
        <div
            class="flex w-full max-w-md flex-col gap-1 rounded-xl border border-border p-1.5"
        >
            <input
                aria-label="Question"
                placeholder="Start typing a question…"
                class="h-9 rounded-md bg-transparent px-3 text-sm outline-none"
                :value=$(query.get())
                @input=$(|e: Event| query.set(e.target.value))
            >
            matches(query: rows)
        </div>
    })
}
