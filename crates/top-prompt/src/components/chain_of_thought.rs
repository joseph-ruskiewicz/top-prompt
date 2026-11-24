use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

/// Classes that animate the content height when a step opens or closes, as the
/// built-in accordion does.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0",
    "[&::details-content]:overflow-hidden",
    "[&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete]",
    "[&[open]::details-content]:h-auto",
);

/// A model's reasoning as a sequence of steps that each open on their own, joined by a
/// line down the left.
///
/// Pass [`chain_of_thought_step`]s as child content. The line runs from each step to
/// the next; the last step has none. Steps rendered in a loop need a `#[key(...)]`,
/// because each keeps its own open state.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     chain_of_thought(
///         chain_of_thought_step(
///             chain_of_thought_trigger("Read the question")
///             chain_of_thought_content(
///                 chain_of_thought_item("The user wants a Rust example.")
///             )
///         )
///         chain_of_thought_step(
///             chain_of_thought_trigger("Pick an approach")
///             chain_of_thought_content(
///                 chain_of_thought_item("An iterator chain is shortest.")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn chain_of_thought(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The [`chain_of_thought_step`]s.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-slot="chain-of-thought"
            class=(class!("flex w-full flex-col", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// One step of a [`chain_of_thought`]: a collapsible `<details>` element.
///
/// Pass a [`chain_of_thought_trigger`] and a [`chain_of_thought_content`] as child
/// content. With the browser runtime, the reader's choice to open or close the step
/// survives a live region rendering it again.
///
/// `attrs` are forwarded to the `<details>`. Extra classes are added to its classes.
#[component]
pub async fn chain_of_thought_step(
    cx: &Cx,
    /// Whether the step starts open.
    #[default]
    default_open: bool,
    /// Extra attributes for the `<details>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`chain_of_thought_trigger`] and a [`chain_of_thought_content`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    let open = signal(cx, || default_open);

    Ok(view! {
        // The wrapper holds the connector: content inside a closed `<details>` other
        // than its summary is hidden, and the line must show either way.
        <div class="group/step relative pb-3 last:pb-0">
            <span
                aria-hidden="true"
                class="absolute top-6 bottom-0 left-[7px] w-px bg-border group-last/step:hidden"
            ></span>
            <details
                id=(&id)
                class=(class!("group/thought", ANIMATION, attrs.remove("class")))
                :open=$(open.get())
                @toggle=$(|_e: Event| {
                    // The details element's open state is outside the expression
                    // vocabulary.
                    open.set(
                        raw!(
                            "cx.hydrate(document.getElementById(${id}.toString()).open)",
                            false,
                        ),
                    )
                })
                (attrs)
            >
                (child)
            </details>
        </div>
    })
}

/// The summary line of a [`chain_of_thought_step`]: a marker, the step's title and a
/// chevron.
///
/// `leading_icon` replaces the default dot marker. `attrs` are forwarded to the
/// `<summary>`. Extra classes are added to its classes.
#[component]
pub async fn chain_of_thought_trigger(
    cx: &Cx,
    /// An icon to use as the marker instead of a dot.
    #[default]
    leading_icon: Option<IconData>,
    /// Extra attributes for the `<summary>` element.
    #[default]
    mut attrs: Attributes,
    /// The step's title.
    #[default]
    child: Child<'_>,
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
            <span
                aria-hidden="true"
                class="flex size-4 shrink-0 items-center justify-center"
            >
                match leading_icon {
                    Some(data) => icon(data: data),
                    None => <span class="size-1.5 rounded-full bg-current"></span>,
                }
            </span>
            <span>(child)</span>
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="shrink-0 transition-transform duration-200 ease-out group-open/thought:rotate-180"
                }
            )
        </summary>
    })
}

/// The body of a [`chain_of_thought_step`], indented past the marker.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn chain_of_thought_content(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// [`chain_of_thought_item`]s, or any other content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("mt-2 flex flex-col gap-2 pl-6", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// One thought in a [`chain_of_thought_content`], in muted small text.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn chain_of_thought_item(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The thought.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "text-sm leading-relaxed text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
