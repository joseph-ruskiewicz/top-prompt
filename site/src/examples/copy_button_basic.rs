use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::copy_button::copy_button;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let invite = "https://example.com/invite/7Q2-KD9";

    Ok(view! {
        <div
            class="flex items-center gap-2 rounded-lg border border-border py-1 pr-1 pl-3 text-sm"
        >
            <span class="font-mono text-muted-foreground">(invite)</span>
            copy_button(text: invite, label: "Copy invite link")
        </div>
    })
}
