use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::message::{MessageRole, message, message_avatar, message_content};

/// A small portrait, inlined so the example needs no image file.
const PORTRAIT: &str = "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' \
    viewBox='0 0 32 32'%3E%3Crect width='32' height='32' fill='%23fcd34d'/%3E%3Ccircle \
    cx='16' cy='13' r='6' fill='%2378350f'/%3E%3Cpath d='M5 32c1-7 6-10 11-10s10 3 11 \
    10z' fill='%2378350f'/%3E%3C/svg%3E";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-6">
            message(
                role: MessageRole::User,
                // The image covers the initials. The message names its author nearby, so
                // `alt` stays empty here; give it text when nothing else does.
                message_avatar(src: Some(PORTRAIT), fallback: "AM")
                message_content("Is it too late to plant garlic?")
            )
            message(
                // The image can't load, so the initials stay.
                message_avatar(
                    src: Some("/no-such-avatar.png"),
                    fallback: "GB",
                    alt: "Garden Bot",
                    attrs: attributes! { class="ring-2 ring-border" }
                )
                message_content("Not yet: plant cloves until the ground freezes.")
            )
        </div>
    })
}
