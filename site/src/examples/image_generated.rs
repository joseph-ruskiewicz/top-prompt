use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    image::image,
    message::{MessageRole, message, message_content},
    thinking_bar::thinking_bar,
};

/// Stands in for an image model. Returns the image as base64, the way many APIs do.
#[procedure]
async fn generate(cx: &Cx) -> Result<String> {
    tokio::time::sleep(Duration::from_millis(2000)).await;
    Ok("PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMjAgMjAwIj48cmVjdCB3aWR0aD0iMzIwIiBoZWlnaHQ9IjIwMCIgZmlsbD0iI2U3ZTVlNCIvPjxjaXJjbGUgY3g9IjIzNiIgY3k9IjYyIiByPSIyNiIgZmlsbD0iI2ZhZmFmOSIvPjxwYXRoIGQ9Ik0wIDIwMCA5MiA5Nmw1OCA2NiA0MC0zOCAxMzAgNzZ6IiBmaWxsPSIjNzg3MTZjIi8+PHBhdGggZD0iTTAgMjAwIDcwIDE0MGw1MCA2MHoiIGZpbGw9IiM1NzUzNGUiLz48L3N2Zz4=".to_owned())
}

#[shard]
async fn reply(cx: &Cx) -> Result<impl View> {
    // Empty while the image is being made.
    let data = signal(cx, || None::<String>);
    let generating = signal(cx, || false);
    let current = data.get();

    Ok(view! {
        <div class="flex w-full flex-col gap-4">
            message(
                role: MessageRole::User,
                message_content("Draw hills under a pale sun.")
            )
            message(
                message_content(
                    if generating.get() {
                        thinking_bar(text: "Drawing the hills")
                    } else {
                        match &current {
                            // Known dimensions give the placeholder the right shape, so
                            // nothing below it jumps when the image arrives.
                            Some(base64) => image(
                                base64: Some(base64.as_str()),
                                media_type: "image/svg+xml",
                                alt: "Grey hills under a pale sun",
                                width: Some(320),
                                height: Some(200),
                                attrs: attributes! { class="max-w-xs" }
                            ),
                            None => "Ready when you are.",
                        }
                    }
                )
            )
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(generating.get())
                    @click=$(async |_e: Event| {
                        generating.set(true);
                        data.set(Some(generate().await));
                        generating.set(false);
                    })
                },
                "Generate"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { reply() })
}
