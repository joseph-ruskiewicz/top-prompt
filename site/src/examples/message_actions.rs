use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    copy_button::copy_button,
    floating::FloatingSide,
    message::{message, message_action, message_actions, message_content},
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let reply = "A haiku is three lines of five, seven and five syllables.";
    // "", "good" or "bad". A real app would also send the rating to the server.
    let rating = signal(cx, String::new);

    Ok(view! {
        <div class="flex w-full flex-col gap-2">
            message(
                message_content((reply))
                message_actions(
                    label: "Actions for this answer",
                    copy_button(text: reply, label: "Copy message")
                    message_action(
                        label: "Good response",
                        attrs: attributes! {
                            :aria-pressed=$(if rating.get() == "good" {
                                "true"
                            } else {
                                "false"
                            })
                            class="aria-pressed:bg-foreground/10 aria-pressed:text-foreground"
                            @click=$(|_e: Event| rating.set("good".to_owned()))
                        },
                        icon(data: iconify_icon!("lucide:thumbs-up"))
                    )
                    message_action(
                        label: "Bad response",
                        attrs: attributes! {
                            :aria-pressed=$(if rating.get() == "bad" {
                                "true"
                            } else {
                                "false"
                            })
                            class="aria-pressed:bg-foreground/10 aria-pressed:text-foreground"
                            @click=$(|_e: Event| rating.set("bad".to_owned()))
                        },
                        icon(data: iconify_icon!("lucide:thumbs-down"))
                    )
                    // The tooltip goes below this one.
                    message_action(
                        label: "Share",
                        side: FloatingSide::Bottom,
                        icon(data: iconify_icon!("lucide:share"))
                    )
                )
            )
            <p role="status" class="text-xs text-muted-foreground">
                $(if rating.get() == "good" {
                    "Rated good."
                } else if rating.get() == "bad" {
                    "Rated bad."
                } else {
                    ""
                })
            </p>
        </div>
    })
}
