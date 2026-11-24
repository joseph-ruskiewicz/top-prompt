use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, Expr},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::{
    button::{ButtonSize, ButtonVariant, button},
    spinner::spinner,
};

/// Classes for the input card. The whole card shows the focus ring while its text
/// area has focus, so the ring surrounds the toolbar too.
const PROMPT_INPUT: StaticClass = class!(
    "flex w-full flex-col rounded-xl border border-border bg-background text-foreground",
    "shadow-xs transition-shadow",
    "has-[textarea:focus-visible]:ring-2 has-[textarea:focus-visible]:ring-ring",
    "has-[textarea:focus-visible]:ring-offset-2",
    "has-[textarea:focus-visible]:ring-offset-background",
    "has-[textarea:disabled]:opacity-50",
);

/// A message composer: a `<form>` holding a [`prompt_input_textarea`] and a
/// [`prompt_input_toolbar`].
///
/// Submitting the form is the app's job: give it an `action`, or a runtime `@submit`
/// handler that prevents the default and sends the message. Pressing Enter in the text
/// area submits the form; Shift+Enter inserts a line break.
///
/// `attrs` are forwarded to the `<form>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     prompt_input(
///         attrs: attributes! {
///             @submit=$(async |e: Event| {
///                 e.prevent_default();
///                 send(draft.get()).await;
///                 draft.set("".to_owned());
///             })
///         },
///         prompt_input_textarea(
///             attrs: attributes! {
///                 name="message"
///                 placeholder="Ask anything"
///                 :value=$(draft.get())
///                 @input=$(|e: Event| draft.set(e.target.value))
///             }
///         )
///         prompt_input_toolbar(
///             prompt_input_tools(
///                 prompt_input_button(
///                     attrs: attributes! { aria-label="Attach files" },
///                     icon(data: iconify_icon!("lucide:paperclip"))
///                 )
///             )
///             prompt_input_submit(
///                 status: PromptStatus::Ready,
///                 disabled: $(draft.get().trim().is_empty())
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn prompt_input(
    cx: &Cx,
    /// Extra attributes for the `<form>` element.
    #[default]
    mut attrs: Attributes,
    /// The text area and toolbar.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <form class=(class!(PROMPT_INPUT, attrs.remove("class"))) (attrs)>(child)</form>
    })
}

/// Classes for a borderless text area that grows with its content up to a maximum
/// height, then scrolls. Browsers without content sizing keep the minimum height.
const TEXTAREA: StaticClass = class!(
    "field-sizing-content max-h-48 min-h-16 w-full resize-none bg-transparent px-3 pt-3",
    "pb-1 text-sm outline-none placeholder:text-muted-foreground",
    "disabled:cursor-not-allowed",
);

/// The text area of a [`prompt_input`].
///
/// Pass the initial text as children. Enter submits the enclosing form and Shift+Enter
/// inserts a line break. Enter does nothing special while an input method is composing
/// text. Submitting with Enter needs the browser runtime; without it, Enter inserts a
/// line break and the submit button still works.
///
/// The text area is labelled "Message" unless `attrs` sets an `aria-label` or
/// `aria-labelledby`. `attrs` are forwarded to the `<textarea>`, with extra classes
/// added to its classes.
#[component]
pub async fn prompt_input_textarea(
    cx: &Cx,
    /// Extra attributes for the `<textarea>` element.
    #[default]
    mut attrs: Attributes,
    /// The initial text.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let labelled = attrs.contains_key("aria-label") || attrs.contains_key("aria-labelledby");

    Ok(view! {
        <textarea
            rows="1"
            aria-label=((!labelled).then_some("Message"))
            class=(class!(TEXTAREA, attrs.remove("class")))
            @keydown=$(|e: Event| {
                // Submitting the form is outside the expression vocabulary. A keydown
                // event only reaches the focused element, so the text area is the
                // document's active element here.
                // Expressions have no `&&`, hence the nested conditions.
                if e.key == "Enter" {
                    if !e.shift_key {
                        if !e.is_composing {
                            e.prevent_default();
                            raw!("document.activeElement?.form?.requestSubmit();", ());
                        }
                    }
                }
            })
            (attrs)
        >
            (child)
        </textarea>
    })
}

/// The row under the text area of a [`prompt_input`], with tools on the left and the
/// submit button on the right.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn prompt_input_toolbar(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`prompt_input_tools`] group and a [`prompt_input_submit`] button.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex items-center justify-between gap-2 p-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A group of tool buttons in a [`prompt_input_toolbar`].
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn prompt_input_tools(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The tool buttons.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex items-center gap-1", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// A ghost button in [`prompt_input_tools`], such as "Attach" or a model picker.
///
/// `size` defaults to `Icon`; use `Sm` for a button with text. Give an icon-only
/// button an `aria-label` in `attrs`. `attrs` are forwarded to the `<button>`, with
/// extra classes added to its classes.
#[component]
pub async fn prompt_input_button(
    cx: &Cx,
    /// The button size.
    #[default(ButtonSize::Icon)]
    size: ButtonSize,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
    /// The button's icon or text.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        button(
            variant: ButtonVariant::Ghost,
            size: size,
            attrs: attributes! {
                if !attrs.contains_key("type") {
                    type="button"
                }
                (attrs)
            },
            (child)
        )
    })
}

/// The state of a chat request, which decides what a [`prompt_input_submit`] shows.
///
/// [`Default`] is `PromptStatus::Ready`, used when no status is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PromptStatus {
    /// Waiting for the next message.
    #[default]
    Ready,
    /// A message was sent and no response has arrived yet.
    Submitted,
    /// A response is arriving.
    Streaming,
    /// The last request failed.
    Error,
}

/// The submit button of a [`prompt_input`].
///
/// What it shows follows `status`:
///
/// - `Ready`: an arrow; the button submits the form.
/// - `Submitted`: a spinner; the button is a plain button labelled "Stop".
/// - `Streaming`: a square; the button is a plain button labelled "Stop".
/// - `Error`: a retry arrow; the button submits the form.
///
/// While it is a plain button, attach the cancel action as an `@click` handler in
/// `attrs`. `disabled` applies in the `Ready` and `Error` states, typically a runtime
/// expression that is `true` while the text area is empty. `attrs` are forwarded to the
/// `<button>`, with extra classes added to its classes.
#[component]
pub async fn prompt_input_submit(
    cx: &Cx,
    /// The state of the request.
    #[default]
    status: PromptStatus,
    /// Whether the button is disabled while it submits.
    #[into]
    #[default(Expr::from(false))]
    disabled: Expr<bool>,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    let submits = matches!(status, PromptStatus::Ready | PromptStatus::Error);

    Ok(view! {
        button(
            size: ButtonSize::Icon,
            attrs: attributes! {
                type=(if submits { "submit" } else { "button" })
                if submits {
                    :disabled=(disabled)
                }
                aria-label=(match status {
                    PromptStatus::Ready => "Send message",
                    PromptStatus::Submitted => "Stop",
                    PromptStatus::Streaming => "Stop generating",
                    PromptStatus::Error => "Retry",
                })
                (attrs)
            },
            match status {
                PromptStatus::Ready => icon(data: iconify_icon!("lucide:arrow-up")),
                PromptStatus::Submitted => spinner(label: ""),
                PromptStatus::Streaming => {
                    icon(
                        data: iconify_icon!("lucide:square"),
                        attrs: attributes! { class="fill-current" }
                    )
                }
                PromptStatus::Error => icon(data: iconify_icon!("lucide:rotate-ccw")),
            }
        )
    })
}
