use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::attachment::attachment_remove;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let removed = signal(cx, || false);

    Ok(view! {
        // A chip for a pasted link rather than a file, with the same remove button.
        <div
            class="flex items-center gap-2 rounded-lg border border-border bg-background py-1 pr-1 pl-3 text-sm"
            :hidden=$(removed.get())
        >
            icon(
                data: iconify_icon!("lucide:link"),
                attrs: attributes! { class="text-muted-foreground" }
            )
            <span class="max-w-56 truncate">
                "https://example.com/recipes/focaccia"
            </span>
            attachment_remove(
                name: "the focaccia link",
                attrs: attributes! { @click=$(|_e: Event| removed.set(true)) }
            )
        </div>
    })
}
