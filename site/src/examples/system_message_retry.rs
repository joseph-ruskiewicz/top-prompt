use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    icon::iconify::iconify_icon,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::system_message::{SystemMessageVariant, system_message, system_message_cta};

/// Stands in for sending the request again.
#[procedure]
async fn retry(cx: &Cx) -> Result<bool> {
    tokio::time::sleep(Duration::from_millis(1200)).await;
    Ok(true)
}

/// The notice re-renders with each change of state.
#[shard]
async fn notice(cx: &Cx) -> Result<impl View> {
    // "failed", "retrying", "done" or "dismissed".
    let state = signal(cx, || "failed".to_owned());

    Ok(view! {
        <div class="flex min-h-14 w-full flex-col">
            match state.get().as_str() {
                "failed" => system_message(
                    variant: SystemMessageVariant::Error,
                    fill: true,
                    "The reply was cut off: the connection dropped."
                    system_message_cta(
                        attrs: attributes! {
                            @click=$(async |_e: Event| {
                                state.set("retrying".to_owned());
                                let ok = retry().await;
                                state.set(
                                    if ok { "done".to_owned() } else { "failed".to_owned() },
                                );
                            })
                        },
                        "Retry"
                    )
                ),
                "retrying" => system_message(
                    leading_icon: Some(iconify_icon!("lucide:loader-circle")),
                    attrs: attributes! { class="[&_svg]:animate-spin" },
                    "Sending the message again…"
                ),
                "done" => system_message(
                    leading_icon: Some(iconify_icon!("lucide:circle-check")),
                    "Reconnected. The reply is complete."
                    system_message_cta(
                        attrs: attributes! { @click=$(|_e: Event| state.set("dismissed".to_owned())) },
                        "Dismiss"
                    )
                ),
                _ => <button
                    type="button"
                    class="self-start text-sm text-muted-foreground underline underline-offset-4"
                    @click=$(|_e: Event| state.set("failed".to_owned()))
                >
                    "Drop the connection again"
                </button>,
            }
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { notice() })
}
