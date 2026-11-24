use topcoat::{
    Result,
    context::Cx,
    router::query_params,
    view::{View, attributes, component, view},
};

use crate::components::suggestion::{suggestion, suggestions};

#[query_params]
struct Query {
    ask: Option<String>,
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // What the form sent. It comes from the URL, so treat it as user input.
    let asked = query_params::<Query>(cx)
        .ok()
        .and_then(|query| query.ask.clone());

    Ok(view! {
        <div class="flex w-full flex-col items-center gap-3">
            // Submit buttons with a `name` send their prompt with the form: no JavaScript,
            // and the page could answer straight away.
            <form method="get" action="#suggestion_submit">
                suggestions(
                    attrs: attributes! { class="justify-center" },
                    for prompt in [
                        "What is a good first houseplant?",
                        "How often do I water a cactus?",
                    ] {
                        suggestion(
                            prompt: prompt,
                            attrs: attributes! { type="submit" name="ask" }
                        )
                    }
                )
            </form>
            if let Some(asked) = asked {
                <p class="text-sm text-muted-foreground">
                    "You asked: "
                    (asked)
                </p>
            }
        </div>
    })
}
