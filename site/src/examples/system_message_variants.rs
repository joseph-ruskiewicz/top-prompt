use topcoat::{
    Result,
    context::Cx,
    icon::iconify::iconify_icon,
    view::{View, attributes, component, view},
};

use crate::components::system_message::{SystemMessageVariant, system_message, system_message_cta};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            // Action is the default variant.
            system_message(
                variant: SystemMessageVariant::Action,
                "This conversation is shared with your team."
            )
            system_message(
                fill: true,
                leading_icon: Some(iconify_icon!("lucide:sparkles")),
                "A newer model is available for this chat."
                system_message_cta("Switch")
            )
            system_message(
                variant: SystemMessageVariant::Warning,
                "You have 3 messages left today."
                // With an `href`, the action is a link.
                system_message_cta(
                    attrs: attributes! { href="/docs/system_message" },
                    "See plans"
                )
            )
            system_message(
                variant: SystemMessageVariant::Error,
                fill: true,
                "The response was cut off: the connection dropped."
                system_message_cta("Retry")
            )
        </div>
    })
}
