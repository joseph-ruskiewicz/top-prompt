use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, component, view},
};

use crate::components::conversation::conversation_scroll_button;

const LINES: [&str; 14] = [
    "Resolving dependencies",
    "Downloading 42 crates",
    "Compiling serde v1.0.219",
    "Compiling tokio v1.47.1",
    "Compiling hyper v1.7.0",
    "Compiling axum v0.8.4",
    "Compiling topcoat v0.12.0",
    "Compiling garden-planner v0.3.1",
    "Running 31 tests",
    "31 passed, 0 failed",
    "Building the container image",
    "Pushing image to the registry",
    "Rolling out 3 replicas",
    "Deploy finished in 2m 41s",
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let at_bottom = signal(cx, || true);

    Ok(view! {
        // A log of your own: a reversed column that starts at the bottom, and the scroll
        // button pointed at it by ID. Scroll up to show the button.
        <div
            id="deploy-log"
            class="flex h-44 w-full max-w-md flex-col-reverse overflow-y-auto rounded-xl border border-border bg-muted/40 p-3 font-mono text-xs"
            @scroll=$(|_e: Event| {
                at_bottom.set(
                    raw!("cx.hydrate(${_e}.inner.currentTarget.scrollTop > -24)", true),
                )
            })
        >
            conversation_scroll_button(
                target: "deploy-log",
                visible: $(!at_bottom.get())
            )
            <div class="flex flex-col gap-1">
                for line in LINES {
                    <p>(line)</p>
                }
            </div>
        </div>
    })
}
