use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, View, attributes, component, view},
};

use super::{
    button::{ButtonSize, ButtonVariant, button},
    tooltip::{tooltip, tooltip_content},
};

/// A ghost icon button that copies `text` to the clipboard.
///
/// After a successful copy, the copy icon turns into a check mark and the tooltip reads
/// "Copied" for two seconds. `label` names the action for the tooltip and for
/// assistive technology. Copying needs the browser runtime and a secure context
/// (HTTPS or `localhost`); without them the button renders but does nothing.
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     copy_button(text: &message.text, label: "Copy message")
/// }
/// ```
#[component]
pub async fn copy_button(
    cx: &Cx,
    /// The text written to the clipboard.
    #[into]
    text: String,
    /// The tooltip and accessible name of the button.
    #[into]
    #[default(String::from("Copy"))]
    label: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    let copied = signal(cx, || false);

    Ok(view! {
        tooltip(
            button(
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Icon,
                attrs: attributes! {
                    type="button"
                    :aria-label=$(if copied.get() {
                        "Copied".to_owned()
                    } else {
                        label.clone()
                    })
                    @click=$(async |_e: Event| {
                        // The clipboard and timers are outside the expression
                        // vocabulary. Handlers only run in the browser, so the Rust
                        // fallbacks exist for type checking. Raw statements carry
                        // their own semicolons, and stay out of `if` blocks, which
                        // compile to non-async functions where `await` is invalid.
                        let written = raw!(
                            "cx.hydrate(await navigator.clipboard.writeText(${text}.toString()).then(() => true, () => false))",
                            true,
                        );
                        copied.set(written);
                        raw!(
                            "await new Promise((resolve) => setTimeout(resolve, 2000));",
                            (),
                        );
                        copied.set(false);
                    })
                    (attrs)
                },
                <span class="contents" :hidden=$(copied.get())>
                    icon(data: iconify_icon!("lucide:copy"))
                </span>
                <span class="contents" :hidden=$(!copied.get())>
                    icon(data: iconify_icon!("lucide:check"))
                </span>
            )
            tooltip_content(
                $(if copied.get() { "Copied".to_owned() } else { label.clone() })
            )
        )
    })
}
