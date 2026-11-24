use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    feedback_bar::{
        feedback_bar, feedback_bar_close, feedback_bar_helpful, feedback_bar_not_helpful,
    },
    message::{message, message_content},
};

/// Stands in for storing the rating. The value comes from the browser, so check it.
#[procedure]
async fn record_rating(cx: &Cx, value: String) -> Result<String> {
    Ok(match value.as_str() {
        "helpful" => "Glad it helped. Your rating was saved.",
        "not_helpful" => "Sorry about that. Your rating was saved, and we'll look at it.",
        _ => "",
    }
    .to_owned())
}

#[shard]
async fn rated_answer(cx: &Cx) -> Result<impl View> {
    // `None` until the reader picks a button, then what the server said, which is empty
    // when they closed the bar.
    let outcome = signal(cx, || None::<String>);

    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            message(
                message_content(
                    "Repot in spring, one size up, into a pot with drainage holes. Water \
                    it well and keep it out of direct sun for a week."
                )
            )
            match outcome.get() {
                None => {
                    feedback_bar(
                        title: "Did this answer your question?",
                        // Each button carries its value: "helpful", "not_helpful" or "close".
                        feedback_bar_helpful(
                            label: "Yes, it did",
                            attrs: attributes! {
                                @click=$(async |e: Event| {
                                    outcome.set(
                                        Some(record_rating(e.current_target.value).await),
                                    )
                                })
                            }
                        )
                        feedback_bar_not_helpful(
                            label: "No, it didn't",
                            attrs: attributes! {
                                @click=$(async |e: Event| {
                                    outcome.set(
                                        Some(record_rating(e.current_target.value).await),
                                    )
                                })
                            }
                        )
                        feedback_bar_close(
                            label: "Don't ask again",
                            attrs: attributes! {
                                @click=$(async |e: Event| {
                                    outcome.set(
                                        Some(record_rating(e.current_target.value).await),
                                    )
                                })
                            }
                        )
                    )
                }
                Some(text) => {
                    if !text.is_empty() {
                        <p role="status" class="text-sm text-muted-foreground">
                            (text)
                        </p>
                    }
                }
            }
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { rated_answer() })
}
