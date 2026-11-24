use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, Expr, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// Classes for the scrolling log.
///
/// The column is reversed, so the browser measures the scroll position from the
/// bottom: while the reader is at the bottom, new content keeps them there, and once
/// they scroll up, their position holds. This needs no JavaScript. `scroll-smooth`
/// animates the jump back to the bottom.
const CONVERSATION: StaticClass = class!(
    "relative flex min-h-0 flex-1 flex-col-reverse overflow-y-auto scroll-smooth",
    "outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
);

/// A scrolling conversation log that sticks to the newest message.
///
/// Pass a [`conversation_content`] as child content. The log scrolls on its own and
/// fills the height its parent gives it, so place it in a flex column or give it a
/// height. While the reader is at the bottom, streamed content keeps them there; once
/// they scroll up, the log stops following and a [`conversation_scroll_button`]
/// appears to jump back down. Short conversations start at the top.
///
/// The log has `role="log"`, so assistive technology announces new messages. Set
/// `aria-busy="true"` in `attrs` while a response streams, so it is read once complete.
///
/// `attrs` are forwarded to the scrolling `<div>`. Extra classes are added to its
/// classes.
///
/// ```ignore
/// view! {
///     <main class="flex h-dvh flex-col">
///         conversation(
///             attrs: attributes! { aria-busy=(if streaming { "true" } else { "false" }) },
///             conversation_content(
///                 for message in messages {
///                     message(role: message.role, message_content((message.text)))
///                 }
///             )
///         )
///         prompt_input(...)
///     </main>
/// }
/// ```
#[component]
pub async fn conversation(
    cx: &Cx,
    /// Extra attributes for the scrolling `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The conversation's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    let at_bottom = signal(cx, || true);

    Ok(view! {
        <div
            id=(&id)
            role="log"
            aria-live="polite"
            tabindex="0"
            class=(class!(CONVERSATION, attrs.remove("class")))
            @scroll=$(|_e: Event| {
                // The reversed column puts the bottom at a scroll offset of zero and
                // counts upward in negative values. Reading the offset needs the element
                // itself, which the expression vocabulary cannot reach.
                at_bottom.set(
                    raw!(
                        "cx.hydrate(document.getElementById(${id}.toString()).scrollTop > -48)",
                        true,
                    ),
                )
            })
            (attrs)
        >
            // The first child sits at the bottom of the reversed column.
            conversation_scroll_button(target: &id, visible: $(!at_bottom.get()))
            <div class="flex shrink-0 grow flex-col">(child)</div>
        </div>
    })
}

/// The centered column of messages inside a [`conversation`].
///
/// Pass messages as children. Content is limited to a readable width and spaced
/// evenly. `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn conversation_content(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The messages.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "mx-auto flex w-full max-w-3xl flex-1 flex-col gap-8 px-4 py-6",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A placeholder for a [`conversation`] without messages, centered in the log.
///
/// Shows `title` and an optional `description`. Pass extra content, such as an icon
/// or suggestions, as children; they appear below the text.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     conversation_empty_state(
///         title: "How can I help?",
///         description: "Ask about your codebase, or pick a suggestion.",
///         suggestions(...)
///     )
/// }
/// ```
#[component]
pub async fn conversation_empty_state(
    cx: &Cx,
    /// The heading.
    #[into]
    #[default(String::from("No messages yet"))]
    title: String,
    /// Supporting text under the heading.
    #[into]
    #[default]
    description: String,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// Content shown below the text.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <div class="flex flex-col gap-1.5">
                <h2 class="text-lg leading-none font-semibold">(title)</h2>
                if !description.is_empty() {
                    <p class="text-sm text-muted-foreground">(description)</p>
                }
            </div>
            (child)
        </div>
    })
}

/// The button that scrolls a [`conversation`] back to its newest message.
///
/// [`conversation`] renders one itself; use this directly only when building a custom
/// log. `target` is the ID of the reversed-column scroll container. `visible` is
/// usually a runtime expression that is `true` while the reader is scrolled up. The
/// button floats at the bottom of the visible area.
#[component]
pub async fn conversation_scroll_button(
    cx: &Cx,
    /// The ID of the scroll container.
    target: &str,
    /// Whether the button shows.
    #[into]
    visible: Expr<bool>,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    let target = target.to_owned();

    Ok(view! {
        // A zero-height strip that sticks to the bottom of the visible area, so the
        // button floats without taking space in the log.
        <div
            class="pointer-events-none sticky bottom-0 z-10 flex h-0 shrink-0 justify-center"
        >
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Icon,
                attrs: attributes! {
                    type="button"
                    aria-label="Scroll to latest message"
                    class="pointer-events-auto absolute bottom-4 bg-background shadow-sm"
                    :hidden=$(!visible)
                    @click=$(|_e: Event| {
                        raw!(
                            "document.getElementById(${target}.toString()).scrollTo({ top: 0 });",
                            (),
                        )
                    })
                    (attrs)
                },
                icon(data: iconify_icon!("lucide:arrow-down"))
            )
        </div>
    })
}
