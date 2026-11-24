use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    loader::{LoaderSize, LoaderVariant, loader},
    message::{message, message_content},
};

/// Stands in for the model's time to first token.
#[procedure]
async fn first_token(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(2500)).await;
    Ok(())
}

#[shard]
async fn reply(cx: &Cx) -> Result<impl View> {
    let waiting = signal(cx, || false);

    Ok(view! {
        <div class="flex w-full flex-col gap-4">
            message(
                message_content(
                    if waiting.get() {
                        // `label` is shown by the Text variants and read out by screen
                        // readers for all of them.
                        loader(
                            variant: LoaderVariant::Text,
                            size: LoaderSize::Sm,
                            label: "Checking the timetable…",
                            attrs: attributes! { class="py-1" }
                        )
                    } else {
                        "The next train to Lisbon leaves at 14:05 from platform 3."
                    }
                )
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(waiting.get())
                    @click=$(async |_e: Event| {
                        waiting.set(true);
                        first_token().await;
                        waiting.set(false);
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
