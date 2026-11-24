use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, View, attributes, class, component, view},
};

use super::{
    button::{ButtonSize, ButtonVariant, button},
    text_shimmer::text_shimmer,
};

/// A row saying the model is thinking, with an optional button to stop thinking and
/// answer right away.
///
/// `text` shimmers while the bar shows. When `stop_label` is given, a small ghost
/// button with that label follows it; wire its click in `stop_attrs`, for example with
/// a runtime `@click` handler that calls a procedure. The row has `role="status"`, so a
/// change of text is announced.
///
/// `attrs` are forwarded to the outer `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     thinking_bar(
///         text: "Planning the answer",
///         stop_label: Some("Answer now"),
///         stop_attrs: attributes! {
///             @click=$(async |_e: Event| { answer_now().await; })
///         }
///     )
/// }
/// ```
#[component]
pub async fn thinking_bar(
    cx: &Cx,
    /// The shimmering label.
    #[into]
    #[default(String::from("Thinking"))]
    text: String,
    /// The label of the stop button, which only shows when this is given.
    #[default]
    stop_label: Option<&str>,
    /// Extra attributes for the stop `<button>`, such as its click handler.
    #[default]
    stop_attrs: Attributes,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-slot="thinking-bar"
            class=(class!(
                "flex w-full items-center justify-between gap-3 text-sm",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span role="status" class="flex min-w-0 items-center gap-2">
                icon(
                    data: iconify_icon!("lucide:sparkles"),
                    attrs: attributes! { class="shrink-0 text-muted-foreground" }
                )
                text_shimmer(attrs: attributes! { class="truncate" }, (text))
            </span>
            if let Some(stop_label) = stop_label {
                button(
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        if !stop_attrs.contains_key("type") {
                            type="button"
                        }
                        class="shrink-0"
                        (stop_attrs)
                    },
                    (stop_label)
                    icon(data: iconify_icon!("lucide:chevron-right"))
                )
            }
        </div>
    })
}
