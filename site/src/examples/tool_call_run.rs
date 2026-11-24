use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    tool_call::{ToolStatus, tool_call, tool_call_input, tool_call_output},
};

/// Stands in for the agent's backend: waits as long as one step of a real call might.
#[procedure]
async fn wait(cx: &Cx, ms: usize) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(ms.min(3000) as u64)).await;
    Ok(())
}

/// The card re-renders on the server each time `step` changes, with the status that
/// step stands for. In an app, the status would come from the agent's run.
#[shard]
async fn run(cx: &Cx) -> Result<impl View> {
    let step = signal(cx, || 0usize);
    let status = match step.get() {
        0 => ToolStatus::Pending,
        1 => ToolStatus::Running,
        _ => ToolStatus::Completed,
    };

    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            tool_call(
                name: "search_trains",
                status: status,
                // A card the reader opened stays open while the status changes.
                tool_call_input(
                    label: "Arguments",
                    "{ \"from\": \"Porto\", \"to\": \"Lisbon\", \"date\": \"Saturday\" }"
                )
                if step.get() == 2 {
                    tool_call_output(
                        "6 departures, 07:00 to 19:00. Fastest: 2 h 49 min."
                    )
                }
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    :disabled=$(step.get() == 1)
                    @click=$(async |_e: Event| {
                        step.set(0);
                        wait(700).await;
                        step.set(1);
                        wait(1800).await;
                        step.set(2);
                    })
                },
                $(if step.get() == 0 { "Run the tool" } else { "Run again" })
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { run() })
}
