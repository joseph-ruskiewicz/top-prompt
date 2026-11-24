use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

use super::text_shimmer::text_shimmer;

/// Classes that animate the content height when the section opens or closes, as the
/// built-in accordion does. Browsers without `::details-content` support open and close
/// without animation.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0",
    "[&::details-content]:overflow-hidden",
    "[&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete]",
    "[&[open]::details-content]:h-auto",
);

/// The model's reasoning, collapsed behind a "Thought for N seconds" trigger.
///
/// Uses a native `<details>` element, so it opens and closes without JavaScript. While
/// `streaming` is `true`, the section is open and its trigger shimmers "Thinking…". Once
/// `streaming` is `false`, it renders closed and the trigger shows `duration_secs`.
/// Pass a [`reasoning_content`] as child content.
///
/// With the browser runtime, the reader's choice to open or close the section survives
/// the page or a live region rendering it again, until `streaming` changes.
///
/// `attrs` are forwarded to the `<details>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     reasoning(
///         streaming: false,
///         duration_secs: Some(4),
///         reasoning_content((thoughts))
///     )
/// }
/// ```
#[component]
pub async fn reasoning(
    cx: &Cx,
    /// Whether the reasoning is still arriving.
    #[default]
    streaming: bool,
    /// How long the model reasoned, shown once it is done.
    #[default]
    duration_secs: Option<u32>,
    /// Extra attributes for the `<details>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`reasoning_content`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    // A separate signal for each phase, so the section opens when reasoning starts and
    // closes when it ends, while the reader's own toggles hold within a phase.
    let open = signal(&cx.keyed(streaming), || streaming);

    Ok(view! {
        <details
            id=(&id)
            data-slot="reasoning"
            class=(class!("group/reasoning w-full", ANIMATION, attrs.remove("class")))
            :open=$(open.get())
            @toggle=$(|_e: Event| {
                // The details element's open state is outside the expression vocabulary.
                open.set(
                    raw!(
                        "cx.hydrate(document.getElementById(${id}.toString()).open)",
                        true,
                    ),
                )
            })
            (attrs)
        >
            reasoning_trigger(streaming: streaming, duration_secs: duration_secs)
            (child)
        </details>
    })
}

/// The text of a [`reasoning_trigger`] for a finished reasoning phase.
fn duration_label(duration_secs: Option<u32>) -> String {
    match duration_secs {
        Some(0) => String::from("Thought for less than a second"),
        Some(1) => String::from("Thought for 1 second"),
        Some(seconds) => format!("Thought for {seconds} seconds"),
        None => String::from("Thought for a few seconds"),
    }
}

/// The clickable summary of a [`reasoning`] section.
///
/// [`reasoning`] renders one from its own props; use this directly only in a custom
/// `<details>`. Shows a pulsing "Thinking…" while `streaming`, and otherwise how long
/// the model thought. A chevron shows whether the section is open.
///
/// `attrs` are forwarded to the `<summary>`. Extra classes are added to its classes.
#[component]
pub async fn reasoning_trigger(
    cx: &Cx,
    /// Whether the reasoning is still arriving.
    #[default]
    streaming: bool,
    /// How long the model reasoned.
    #[default]
    duration_secs: Option<u32>,
    /// Extra attributes for the `<summary>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <summary
            class=(class!(
                "flex w-fit cursor-pointer list-none items-center gap-2 rounded-md text-sm",
                "text-muted-foreground outline-none transition-colors hover:text-foreground",
                "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "focus-visible:ring-offset-background [&::-webkit-details-marker]:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(data: iconify_icon!("lucide:brain"))
            if streaming {
                text_shimmer("Thinking…")
            } else {
                <span>(duration_label(duration_secs))</span>
            }
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="transition-transform duration-200 ease-out group-open/reasoning:rotate-180"
                }
            )
        </summary>
    })
}

/// The reasoning text inside a [`reasoning`] section, muted and set off by a rule on
/// its left.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn reasoning_content(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The reasoning text.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "mt-3 ml-2 border-l-2 border-border pl-4 text-sm leading-relaxed",
                "whitespace-pre-wrap text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
