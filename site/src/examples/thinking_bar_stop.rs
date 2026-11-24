use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    message::{message, message_content},
    thinking_bar::thinking_bar,
};

/// Stands in for the model thinking at length before it answers.
#[procedure]
async fn think(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(6000)).await;
    Ok(())
}

/// Stands in for telling the model to stop thinking and answer with what it has.
#[procedure]
async fn answer_now(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(400)).await;
    Ok(())
}

#[shard]
async fn reply(cx: &Cx) -> Result<impl View> {
    // "thinking", "full" or "short": which answer arrived.
    let state = signal(cx, || "full".to_owned());

    Ok(view! {
        <div class="flex w-full max-w-md flex-col gap-4">
            message(
                message_content(
                    match state.get().as_str() {
                        "thinking" => thinking_bar(
                            text: "Comparing train, bus and car",
                            stop_label: Some("Answer now"),
                            stop_attrs: attributes! {
                                @click=$(async |_e: Event| {
                                    answer_now().await;
                                    state.set("short".to_owned());
                                })
                            }
                        ),
                        "short" => "Quick answer: take the train. It is the fastest on most days.",
                        _ => "The train: 2 h 49 min, and cheaper than the bus once you count the \
                            transfer. Driving is only faster before 7:00.",
                    }
                )
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(state.get() == "thinking")
                    @click=$(async |_e: Event| {
                        state.set("thinking".to_owned());
                        think().await;
                        // Unless the reader asked for the answer early.
                        if state.get() == "thinking" {
                            state.set("full".to_owned());
                        }
                    })
                },
                "Ask again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { reply() })
}
