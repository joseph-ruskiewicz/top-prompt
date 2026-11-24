use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{View, component, view},
};

use crate::components::{
    copy_button::copy_button,
    message::{message, message_action, message_actions, message_content},
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let reply = "A haiku is three lines of five, seven and five syllables.";

    Ok(view! {
        <div class="w-full">
            message(
                message_content((reply))
                message_actions(
                    copy_button(text: reply, label: "Copy message")
                    message_action(
                        label: "Good response",
                        icon(data: iconify_icon!("lucide:thumbs-up"))
                    )
                    message_action(
                        label: "Bad response",
                        icon(data: iconify_icon!("lucide:thumbs-down"))
                    )
                    message_action(
                        label: "Read aloud",
                        icon(data: iconify_icon!("lucide:volume-2"))
                    )
                )
            )
        </div>
    })
}
