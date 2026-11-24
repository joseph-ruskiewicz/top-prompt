use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    icon::iconify::iconify_icon,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    steps::{steps, steps_content, steps_item, steps_trigger},
    text_shimmer::text_shimmer,
};

const DONE: [&str; 4] = [
    "Opened src/main.rs",
    "Found the route that returns 500",
    "Ran cargo test: 1 failure",
    "Fixed the missing guard and ran the tests again: all pass",
];

/// Stands in for the agent: waits as long as one operation takes.
#[procedure]
async fn next_step(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(1100)).await;
    Ok(())
}

/// Re-renders as the agent reports each operation.
#[shard]
async fn agent(cx: &Cx) -> Result<impl View> {
    let finished = signal(cx, || DONE.len());
    let count = finished.get().min(DONE.len());
    let running = count < DONE.len();
    let total = DONE.len();

    Ok(view! {
        <div class="flex w-full max-w-md flex-col gap-4">
            // Opens itself for a run; once it has rendered, the reader's own open or close
            // wins over `default_open`.
            steps(
                default_open: true,
                steps_trigger(
                    leading_icon: Some(
                        if running {
                            iconify_icon!("lucide:loader-circle")
                        } else {
                            iconify_icon!("lucide:circle-check")
                        },
                    ),
                    if running {
                        text_shimmer("Fixing the failing route")
                    } else {
                        "Fixed the failing route"
                    }
                )
                steps_content(
                    for item in DONE.iter().take(count) {
                        steps_item((*item))
                    }
                )
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(running)
                    @click=$(async |_e: Event| {
                        finished.set(0);
                        while finished.get() < total {
                            next_step().await;
                            finished.increment();
                        }
                        finished.set(total);
                    })
                },
                "Run the agent again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { agent() })
}
