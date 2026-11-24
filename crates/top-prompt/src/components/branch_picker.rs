use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// A "‹ 2 / 3 ›" control for moving between alternate responses to the same message.
///
/// Add a [`branch_picker_previous`], a [`branch_picker_count`], and a
/// [`branch_picker_next`] as children. It is sized to sit in `message_actions`.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     branch_picker(
///         branch_picker_previous(
///             attrs: attributes! {
///                 disabled=(current == 1)
///                 @click=$(|_e: Event| branch.decrement())
///             }
///         )
///         branch_picker_count(current: current, total: total)
///         branch_picker_next(
///             attrs: attributes! {
///                 disabled=(current == total)
///                 @click=$(|_e: Event| branch.increment())
///             }
///         )
///     )
/// }
/// ```
#[component]
pub async fn branch_picker(
    cx: &Cx,
    /// The accessible name of the group.
    #[into]
    #[default(String::from("Response versions"))]
    label: String,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The previous button, the count, and the next button.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="group"
            aria-label=(label)
            class=(class!("inline-flex items-center", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The button that shows the previous alternate response.
///
/// Attach the action through `attrs`, and set `disabled` there on the first response.
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn branch_picker_previous(
    cx: &Cx,
    /// Extra attributes for the `<button>` element.
    #[default]
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
                aria-label="Previous version"
                (attrs)
            },
            icon(data: iconify_icon!("lucide:chevron-left"))
        )
    })
}

/// The button that shows the next alternate response.
///
/// Attach the action through `attrs`, and set `disabled` there on the last response.
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn branch_picker_next(
    cx: &Cx,
    /// Extra attributes for the `<button>` element.
    #[default]
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
                aria-label="Next version"
                (attrs)
            },
            icon(data: iconify_icon!("lucide:chevron-right"))
        )
    })
}

/// The "2 / 3" position of a [`branch_picker`], read as "Version 2 of 3".
///
/// `attrs` are forwarded to the `<span>`. Extra classes are added to its classes.
#[component]
pub async fn branch_picker_count(
    cx: &Cx,
    /// The 1-based number of the shown response.
    current: usize,
    /// How many responses there are.
    total: usize,
    /// Extra attributes for the `<span>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                // `relative` contains the screen reader text, which would otherwise
                // stretch the page's scroll area.
                "relative min-w-10 text-center text-xs font-medium text-muted-foreground",
                "tabular-nums",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span aria-hidden="true">
                (current)
                " / "
                (total)
            </span>
            <span class="sr-only">
                "Version "
                (current)
                " of "
                (total)
            </span>
        </span>
    })
}
