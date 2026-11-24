use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    text_shimmer::text_shimmer,
};

const STAGES: [&str; 3] = ["Searching the web", "Reading 4 pages", "Writing the answer"];

/// Stands in for the agent moving on to its next stage.
#[procedure]
async fn next_stage(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(1500)).await;
    Ok(())
}

#[shard]
async fn status(cx: &Cx) -> Result<impl View> {
    let stage = signal(cx, || STAGES.len());
    let current = stage.get();
    let total = STAGES.len();

    Ok(view! {
        <div class="flex flex-col items-center gap-4">
            // The shimmer isn't announced by itself, so the status role goes around it.
            <p role="status" class="h-6 text-sm">
                match STAGES.get(current) {
                    Some(text) => text_shimmer(duration_ms: 1500, (*text)),
                    None => "Done: 4 sources, 1 answer.",
                }
            </p>
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    disabled=(current < STAGES.len())
                    @click=$(async |_e: Event| {
                        stage.set(0);
                        while stage.get() < total {
                            next_stage().await;
                            stage.increment();
                        }
                        stage.set(total);
                    })
                },
                "Search again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { status() })
}
