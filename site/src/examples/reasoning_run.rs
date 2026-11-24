use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    reasoning::{reasoning, reasoning_content},
};

const THOUGHTS: [&str; 3] = [
    "The reader is flying Porto to Tokyo with one change.",
    "A two-hour layover in Frankfurt is tight if the first leg is late.",
    "I should suggest the later connection and mention the lounge.",
];

/// Stands in for the model: waits as long as one thought takes.
#[procedure]
async fn next_thought(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(1300)).await;
    Ok(())
}

/// Re-renders as thoughts arrive. While `streaming`, the section is open and shimmers;
/// when it ends, it closes and shows how long the model thought.
#[shard]
async fn thinking(cx: &Cx) -> Result<impl View> {
    let arrived = signal(cx, || THOUGHTS.len());
    let count = arrived.get().min(THOUGHTS.len());
    let streaming = count < THOUGHTS.len();

    Ok(view! {
        <div class="flex w-full flex-col gap-4">
            reasoning(
                streaming: streaming,
                // Measure this in your app; it is fixed here.
                duration_secs: Some(4),
                reasoning_content(
                    for thought in THOUGHTS.iter().take(count) {
                        <p>(*thought)</p>
                    }
                )
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(streaming)
                    @click=$(async |_e: Event| {
                        arrived.set(0);
                        next_thought().await;
                        arrived.set(1);
                        next_thought().await;
                        arrived.set(2);
                        next_thought().await;
                        arrived.set(3);
                    })
                },
                "Think again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { thinking() })
}
