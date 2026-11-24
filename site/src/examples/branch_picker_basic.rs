use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    branch_picker::{
        branch_picker, branch_picker_count, branch_picker_next, branch_picker_previous,
    },
    copy_button::copy_button,
    message::{message, message_actions, message_content},
};

/// Three answers the model gave to the same question.
const VERSIONS: [&str; 3] = [
    "Bake at 180°C for 25 minutes, until a skewer comes out clean.",
    "25 minutes at 180°C. Start checking at 20: the top should spring back when pressed.",
    "In a fan oven, 160°C for about 25 minutes; in a conventional one, 180°C.",
];

/// The message re-renders on the server with the version the reader picked.
#[shard]
async fn answer(cx: &Cx) -> Result<impl View> {
    // The browser sends this back, so keep it in range before using it.
    let shown = signal(cx, || 1usize);
    let current = shown.get().clamp(1, VERSIONS.len());
    let text = VERSIONS[current - 1];

    Ok(view! {
        message(
            message_content((text))
            message_actions(
                branch_picker(
                    branch_picker_previous(
                        attrs: attributes! {
                            disabled=(current == 1)
                            @click=$(|_e: Event| shown.decrement())
                        }
                    )
                    branch_picker_count(current: current, total: VERSIONS.len())
                    branch_picker_next(
                        attrs: attributes! {
                            disabled=(current == VERSIONS.len())
                            @click=$(|_e: Event| shown.increment())
                        }
                    )
                )
                // The copy button copies whichever version is showing.
                copy_button(text: text, label: "Copy message")
            )
        )
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { <div class="w-full">answer()</div> })
}
