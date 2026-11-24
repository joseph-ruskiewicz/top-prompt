use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    chain_of_thought::{
        chain_of_thought, chain_of_thought_content, chain_of_thought_item, chain_of_thought_step,
        chain_of_thought_trigger,
    },
    text_shimmer::text_shimmer,
};

/// Each step of the model's reasoning: a title and what it thought.
const STEPS: [(&str, &str); 3] = [
    (
        "Read the question",
        "They want to split a bill of 84.60 between three people.",
    ),
    (
        "Took out the dessert",
        "One person skipped the 9.60 dessert, so 75.00 is shared.",
    ),
    (
        "Worked out the shares",
        "25.00 each, plus 4.80 each for the two who had dessert.",
    ),
];

/// Stands in for the model taking time to think.
#[procedure]
async fn wait(cx: &Cx, ms: usize) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(ms.min(3000) as u64)).await;
    Ok(())
}

/// Re-renders with each step the model finishes, as a live region would in an app.
#[shard]
async fn thoughts(cx: &Cx) -> Result<impl View> {
    // How many steps have arrived. The last one is in progress until the count goes past
    // the end; the page starts with a finished run.
    let arrived = signal(cx, || STEPS.len() + 1);
    let count = arrived.get().min(STEPS.len());
    let done = arrived.get() > STEPS.len();

    Ok(view! {
        <div class="flex w-full max-w-md flex-col gap-4">
            chain_of_thought(
                // Each step keeps its own open state, so key them.
                #[key(title)]
                for (index, (title, thought)) in STEPS.iter().take(count).enumerate() {
                    let in_progress = !done && index + 1 == count;
                    chain_of_thought_step(
                        default_open: in_progress,
                        chain_of_thought_trigger(
                            if in_progress {
                                text_shimmer((*title))
                            } else {
                                (*title)
                            }
                        )
                        chain_of_thought_content(chain_of_thought_item((*thought)))
                    )
                }
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(!done)
                    @click=$(async |_e: Event| {
                        arrived.set(1);
                        wait(1200).await;
                        arrived.set(2);
                        wait(1200).await;
                        arrived.set(3);
                        wait(1200).await;
                        arrived.set(4);
                    })
                },
                "Think again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { thoughts() })
}
