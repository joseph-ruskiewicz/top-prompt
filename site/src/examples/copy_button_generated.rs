use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    copy_button::copy_button,
};

/// Stands in for creating a share link on the server.
#[procedure]
async fn new_share_code(cx: &Cx, previous: String) -> Result<String> {
    const CODES: [&str; 4] = ["7Q2-KD9", "M4X-P1C", "B8R-ZT3", "H5W-LN6"];
    let index = CODES
        .iter()
        .position(|code| *code == previous)
        .map_or(0, |i| i + 1);
    Ok(CODES[index % CODES.len()].to_owned())
}

/// `text` is fixed when the button renders, so the shard renders it again with each
/// new link.
#[shard]
async fn share_link(cx: &Cx) -> Result<impl View> {
    let code = signal(cx, || "7Q2-KD9".to_owned());
    let link = format!("https://example.com/share/{}", code.get());

    Ok(view! {
        <div class="flex w-full max-w-md items-center gap-2">
            <div
                class="flex min-w-0 flex-1 items-center gap-1 rounded-lg border border-border py-1 pr-1 pl-3 text-sm"
            >
                <span class="flex-1 truncate font-mono text-muted-foreground">
                    (link.as_str())
                </span>
                copy_button(
                    text: &link,
                    label: "Copy share link",
                    attrs: attributes! { class="text-foreground" }
                )
            </div>
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    @click=$(async |_e: Event| code.set(new_share_code(code.get()).await))
                },
                "New link"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { share_link() })
}
