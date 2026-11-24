use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

use super::{
    badge::{BadgeVariant, badge},
    spinner::spinner,
};

/// The progress of a [`tool_call`].
///
/// [`Default`] is `ToolStatus::Pending`, used when no status is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToolStatus {
    /// The model asked for the tool, but it has not started.
    #[default]
    Pending,
    /// The tool is running.
    Running,
    /// The tool finished and returned output.
    Completed,
    /// The tool failed.
    Error,
}

impl ToolStatus {
    /// The status text shown in the badge.
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Error => "Error",
        }
    }

    /// The badge style for the status.
    fn variant(self) -> BadgeVariant {
        match self {
            Self::Pending => BadgeVariant::Outline,
            Self::Running | Self::Completed => BadgeVariant::Secondary,
            Self::Error => BadgeVariant::Destructive,
        }
    }
}

/// Classes that animate the body height when the card opens or closes, as the
/// built-in accordion does.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0",
    "[&::details-content]:overflow-hidden",
    "[&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete]",
    "[&[open]::details-content]:h-auto",
);

/// A collapsible card for one tool call: the tool's name and status, and when opened,
/// its input and output.
///
/// Uses a native `<details>` element and starts closed. Pass a [`tool_call_input`] and
/// a [`tool_call_output`] as children. With the browser runtime, the reader's choice
/// to open or close the card survives the page or a live region rendering it again, so
/// a streaming status update does not close it.
///
/// `attrs` are forwarded to the `<details>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     tool_call(
///         name: "search_docs",
///         status: ToolStatus::Completed,
///         tool_call_input((r#"{ "query": "live regions" }"#))
///         tool_call_output((results_json))
///     )
/// }
/// ```
#[component]
pub async fn tool_call(
    cx: &Cx,
    /// The tool's name.
    #[into]
    name: String,
    /// The progress of the call.
    #[default]
    status: ToolStatus,
    /// Extra attributes for the `<details>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`tool_call_input`] and a [`tool_call_output`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    let open = signal(cx, || false);

    Ok(view! {
        <details
            id=(&id)
            data-slot="tool-call"
            class=(class!(
                "group/tool w-full rounded-lg border border-border bg-card text-card-foreground",
                ANIMATION,
                attrs.remove("class"),
            ))
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
            tool_call_header(name: name, status: status)
            <div class="flex flex-col gap-3 border-t border-border p-3">(child)</div>
        </details>
    })
}

/// The clickable summary of a [`tool_call`]: a wrench, the tool's name, and a status
/// badge.
///
/// [`tool_call`] renders one from its own props; use this directly only in a custom
/// `<details>`. The badge shows a spinner while the tool runs.
///
/// `attrs` are forwarded to the `<summary>`. Extra classes are added to its classes.
#[component]
pub async fn tool_call_header(
    cx: &Cx,
    /// The tool's name.
    #[into]
    name: String,
    /// The progress of the call.
    #[default]
    status: ToolStatus,
    /// Extra attributes for the `<summary>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <summary
            class=(class!(
                "flex cursor-pointer list-none items-center gap-2 rounded-lg px-3 py-2.5",
                "text-sm outline-none transition-colors hover:bg-foreground/5",
                "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "focus-visible:ring-offset-background [&::-webkit-details-marker]:hidden",
                "group-open/tool:rounded-b-none",
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(
                data: iconify_icon!("lucide:wrench"),
                attrs: attributes! { class="shrink-0 text-muted-foreground" }
            )
            <span class="min-w-0 truncate font-mono text-xs font-medium">(name)</span>
            badge(
                variant: status.variant(),
                match status {
                    ToolStatus::Pending => icon(
                        data: iconify_icon!("lucide:circle-dashed")
                    ),
                    ToolStatus::Running => spinner(label: ""),
                    ToolStatus::Completed => icon(
                        data: iconify_icon!("lucide:circle-check")
                    ),
                    ToolStatus::Error => icon(data: iconify_icon!("lucide:circle-x")),
                }
                (status.label())
            )
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="ml-auto shrink-0 text-muted-foreground transition-transform \
                        duration-200 ease-out group-open/tool:rotate-180"
                }
            )
        </summary>
    })
}

/// Classes for the monospace block of a section.
const SECTION_BODY: StaticClass = class!(
    "overflow-x-auto rounded-md p-3 font-mono text-xs leading-relaxed",
    "whitespace-pre-wrap break-words",
);

/// Classes for a section's label.
const SECTION_LABEL: StaticClass =
    class!("text-xs font-medium tracking-wide text-muted-foreground uppercase");

/// The arguments a [`tool_call`] was made with, under an "Input" label.
///
/// Pass the arguments, typically pretty-printed JSON, as children. `attrs` are
/// forwarded to the outer `<div>`. Extra classes are added to its classes.
#[component]
pub async fn tool_call_input(
    cx: &Cx,
    /// The section label.
    #[into]
    #[default(String::from("Input"))]
    label: String,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The arguments.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex flex-col gap-1.5", attrs.remove("class"))) (attrs)>
            <span class=(SECTION_LABEL)>(label)</span>
            <pre class=(class!(SECTION_BODY, "bg-foreground/5"))>(child)</pre>
        </div>
    })
}

/// What a [`tool_call`] returned, under an "Output" label, or its failure under an
/// "Error" label when `error` is `true`.
///
/// Pass the result as children. `attrs` are forwarded to the outer `<div>`. Extra
/// classes are added to its classes.
#[component]
pub async fn tool_call_output(
    cx: &Cx,
    /// Whether the output is an error message.
    #[default]
    error: bool,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The result.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex flex-col gap-1.5", attrs.remove("class"))) (attrs)>
            <span class=(SECTION_LABEL)>(if error { "Error" } else { "Output" })</span>
            <pre
                class=(class!(
                    SECTION_BODY,
                    "bg-destructive/10 text-foreground ring-1 ring-destructive/50 ring-inset" if error else "bg-foreground/5",
                ))
            >
                (child)
            </pre>
        </div>
    })
}
