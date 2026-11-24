use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    view::{Attributes, Child, View, attributes, class, component, element_id, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// A strip asking whether a response helped, with thumbs up, thumbs down and a close
/// button.
///
/// Pass [`feedback_bar_helpful`], [`feedback_bar_not_helpful`] and
/// [`feedback_bar_close`] as child content. The buttons carry the `value`s `helpful`,
/// `not_helpful` and `close`, so one handler can read `e.current_target.value`; what a
/// click does is up to the app. The strip is a group labelled by its title.
///
/// `attrs` are forwarded to the outer `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     feedback_bar(
///         title: "Did this answer help?",
///         feedback_bar_helpful(attrs: attributes! { @click=$(...) })
///         feedback_bar_not_helpful(attrs: attributes! { @click=$(...) })
///         feedback_bar_close(attrs: attributes! { @click=$(...) })
///     )
/// }
/// ```
#[component]
pub async fn feedback_bar(
    cx: &Cx,
    /// The question.
    #[into]
    #[default(String::from("Was this response helpful?"))]
    title: String,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The feedback buttons.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let title_id = format!("{}-title", element_id(cx));

    Ok(view! {
        <div
            role="group"
            aria-labelledby=(&title_id)
            data-slot="feedback-bar"
            class=(class!(
                "flex w-full items-center gap-2 rounded-xl border border-border bg-background",
                "py-1 pr-1 pl-4 text-sm shadow-xs",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span id=(&title_id) class="min-w-0 flex-1 truncate text-muted-foreground">
                (title)
            </span>
            <div class="flex shrink-0 items-center gap-0.5">(child)</div>
        </div>
    })
}

/// A ghost icon button of a [`feedback_bar`].
async fn feedback_button(
    cx: &Cx,
    value: &str,
    label: String,
    data: IconData,
    attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        button(
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Icon,
            attrs: attributes! {
                if !attrs.contains_key("type") {
                    type="button"
                }
                value=(value)
                aria-label=(&label)
                title=(&label)
                (attrs)
            },
            icon(data: data)
        )
    })
}

/// The thumbs-up button of a [`feedback_bar`], with the value `helpful`.
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn feedback_bar_helpful(
    cx: &Cx,
    /// The accessible name.
    #[into]
    #[default(String::from("Helpful"))]
    label: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    feedback_button(
        cx,
        "helpful",
        label,
        iconify_icon!("lucide:thumbs-up"),
        attrs,
    )
    .await
}

/// The thumbs-down button of a [`feedback_bar`], with the value `not_helpful`.
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn feedback_bar_not_helpful(
    cx: &Cx,
    /// The accessible name.
    #[into]
    #[default(String::from("Not helpful"))]
    label: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    feedback_button(
        cx,
        "not_helpful",
        label,
        iconify_icon!("lucide:thumbs-down"),
        attrs,
    )
    .await
}

/// The close button of a [`feedback_bar`], with the value `close`.
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn feedback_bar_close(
    cx: &Cx,
    /// The accessible name.
    #[into]
    #[default(String::from("Dismiss"))]
    label: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    feedback_button(cx, "close", label, iconify_icon!("lucide:x"), attrs).await
}
