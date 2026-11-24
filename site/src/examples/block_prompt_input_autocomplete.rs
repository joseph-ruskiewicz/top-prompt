use topcoat::{
    Result,
    context::Cx,
    router::query_params,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
    suggestion::{suggestion, suggestions},
};

/// The prompts the app can suggest. A real app might look these up in a database.
const PROMPTS: [&str; 8] = [
    "How do I deploy to a VPS?",
    "How do I add a database?",
    "How do I write tests for a handler?",
    "How does streaming work?",
    "What is a live region?",
    "What is a procedure?",
    "Why use server rendering?",
    "Write a contact form",
];

#[query_params]
struct Query {
    q: Option<String>,
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // The typed text arrives as a query parameter, so the server filters and highlights.
    let typed = query_params::<Query>(cx)
        .ok()
        .and_then(|query| query.q.clone())
        .unwrap_or_else(|| "how do".to_owned());
    let needle = typed.trim().to_lowercase();
    let matches: Vec<&str> = PROMPTS
        .into_iter()
        .filter(|prompt| prompt.to_lowercase().contains(&needle))
        .collect();
    let draft = signal(cx, || typed.clone());

    Ok(view! {
        <div class="flex h-full w-full flex-col items-center justify-end p-6">
            <div class="flex w-full max-w-2xl flex-col gap-2">
                if !matches.is_empty() {
                    suggestions(
                        attrs: attributes! {
                            class="flex-col gap-0 rounded-xl border border-border p-1.5"
                        },
                        for prompt in matches {
                            suggestion(
                                prompt: prompt,
                                highlight: Some(needle.as_str()),
                                attrs: attributes! { @click=$(|e: Event| draft.set(e.current_target.value)) }
                            )
                        }
                    )
                }
                // Enter asks the server for matches: the form reloads the page with `?q=`.
                prompt_input(
                    attrs: attributes! { method="get" },
                    prompt_input_textarea(
                        attrs: attributes! {
                            name="q"
                            placeholder="Start typing a question"
                            :value=$(draft.get())
                            @input=$(|e: Event| draft.set(e.target.value))
                        }
                    )
                    prompt_input_toolbar(
                        <span class="px-1 text-xs text-muted-foreground">
                            "Press Enter to update the suggestions"
                        </span>
                        prompt_input_tools()
                        prompt_input_submit()
                    )
                )
            </div>
        </div>
    })
}
