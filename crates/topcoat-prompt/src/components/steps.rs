use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

/// Classes that animate the content height when the list opens or closes, as the
/// built-in accordion does.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0",
    "[&::details-content]:overflow-hidden",
    "[&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete]",
    "[&[open]::details-content]:h-auto",
);

/// A collapsible list of what an agent did: searches, file reads, commands.
///
/// Uses a native `<details>` element, so it opens and closes without JavaScript. Pass a
/// [`steps_trigger`] and a [`steps_content`] as child content. With the browser runtime,
/// the reader's choice to open or close the list survives a live region rendering it
/// again.
///
/// `attrs` are forwarded to the `<details>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     steps(
///         default_open: true,
///         steps_trigger(leading_icon: Some(iconify_icon!("lucide:search")), "Searched 3 sites")
///         steps_content(
///             steps_item("Read the release notes")
///             steps_item("Compared the two changelogs")
///         )
///     )
/// }
/// ```
#[component]
pub async fn steps(
    cx: &Cx,
    /// Whether the list starts open.
    #[default]
    default_open: bool,
    /// Extra attributes for the `<details>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`steps_trigger`] and a [`steps_content`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    let open = signal(cx, || default_open);

    Ok(view! {
        <details
            id=(&id)
            data-slot="steps"
            class=(class!("group/steps w-full", ANIMATION, attrs.remove("class")))
            :open=$(open.get())
            @toggle=$(|_e: Event| {
                // The details element's open state is outside the expression vocabulary.
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
    })
}

/// The summary line of [`steps`], with an optional leading icon and a chevron.
///
/// `attrs` are forwarded to the `<summary>`. Extra classes are added to its classes.
#[component]
pub async fn steps_trigger(
    cx: &Cx,
    /// An icon before the text.
    #[default]
    leading_icon: Option<IconData>,
    /// Extra attributes for the `<summary>` element.
    #[default]
    mut attrs: Attributes,
    /// The summary text.
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
            if let Some(data) = leading_icon {
                icon(data: data, attrs: attributes! { class="shrink-0" })
            }
            <span>(child)</span>
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="shrink-0 transition-transform duration-200 ease-out group-open/steps:rotate-180"
                }
            )
        </summary>
    })
}

/// The body of [`steps`]: a [`steps_bar`] down the left and the items beside it.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
#[component]
pub async fn steps_content(
    cx: &Cx,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// [`steps_item`]s, or any other content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("mt-3 flex gap-3", attrs.remove("class"))) (attrs)>
            steps_bar()
            <div class="flex min-w-0 flex-1 flex-col gap-2">(child)</div>
        </div>
    })
}

/// The vertical rule down the left of a [`steps_content`]. It is decorative and hidden
/// from assistive technology.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn steps_bar(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <div
            aria-hidden="true"
            class=(class!(
                "ml-[7px] w-0.5 shrink-0 rounded-full bg-border",
                attrs.remove("class"),
            ))
            (attrs)
        ></div>
    })
}

/// One operation in a [`steps_content`], in muted small text.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn steps_item(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The description of the operation.
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
