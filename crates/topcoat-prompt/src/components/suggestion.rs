use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// A wrapping row of [`suggestion`] buttons, such as starter prompts under an empty
/// conversation.
///
/// `label` names the group for assistive technology. `attrs` are forwarded to the
/// `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     suggestions(
///         for prompt in ["Summarize this page", "Write a haiku about Rust"] {
///             suggestion(
///                 prompt: prompt,
///                 attrs: attributes! {
///                     @click=$(|e: Event| draft.set(e.current_target.value))
///                 }
///             )
///         }
///     )
/// }
/// ```
#[component]
pub async fn suggestions(
    cx: &Cx,
    /// The accessible name of the group.
    #[into]
    #[default(String::from("Suggestions"))]
    label: String,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The [`suggestion`] buttons.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="group"
            aria-label=(label)
            class=(class!("flex w-full flex-wrap gap-2", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// Classes for a suggestion in highlight mode: a full-width row for autocomplete-style
/// lists, quiet until hovered.
const HIGHLIGHT_ROW: StaticClass = class!(
    "flex w-full cursor-pointer items-center rounded-md px-3 py-2 text-left text-sm",
    "text-muted-foreground transition-colors outline-none hover:bg-foreground/5",
    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
    "focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
);

/// A button carrying a suggested prompt.
///
/// The prompt is the button's `value` and its `data-prompt` attribute, so a click
/// handler can read it from the event, for example with `e.current_target.value`. What
/// a click does is up to the app: fill the text area, or send the prompt right away. As
/// a `type="submit"` button with a `name` inside a form, it submits the prompt without
/// JavaScript.
///
/// The button shows `label`, or the prompt itself when no label is given. By default it
/// is a small outline button. With `highlight`, it is a full-width row for an
/// autocomplete-style list instead: the first match of `highlight` in the text, ignoring
/// ASCII case, is in the foreground color and the rest is muted. `attrs` are forwarded
/// to the `<button>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     suggestions(
///         attrs: attributes! { class="flex-col gap-0" },
///         for prompt in matches {
///             suggestion(prompt: prompt, highlight: Some(&typed))
///         }
///     )
/// }
/// ```
#[component]
pub async fn suggestion(
    cx: &Cx,
    /// The prompt the suggestion stands for.
    #[into]
    prompt: String,
    /// Shorter text to show instead of the prompt.
    #[into]
    #[default]
    label: String,
    /// Text to emphasize, which also switches to the full-width row style.
    #[default]
    highlight: Option<&str>,
    /// Extra attributes for the `<button>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let text = if label.is_empty() {
        prompt.clone()
    } else {
        label
    };

    Ok(view! {
        if let Some(highlight) = highlight {
            let (before, matched, after) = split_match(&text, highlight);
            <button
                type=(if attrs.contains_key("type") { None } else { Some("button") })
                value=(&prompt)
                data-prompt=(&prompt)
                class=(class!(HIGHLIGHT_ROW, attrs.remove("class")))
                (attrs)
            >
                <span class="truncate">
                    (before)
                    <span class="text-foreground">(matched)</span>
                    (after)
                </span>
            </button>
        } else {
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    if !attrs.contains_key("type") {
                        type="button"
                    }
                    value=(&prompt)
                    data-prompt=(&prompt)
                    (attrs)
                },
                (text)
            )
        }
    })
}

/// Splits `text` around the first match of `needle`, ignoring ASCII case. Without a
/// match, all of `text` comes first.
fn split_match<'a>(text: &'a str, needle: &str) -> (&'a str, &'a str, &'a str) {
    if !needle.is_empty() {
        for (start, _) in text.char_indices() {
            let end = start + needle.len();
            if text
                .get(start..end)
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(needle))
            {
                return (&text[..start], &text[start..end], &text[end..]);
            }
        }
    }
    (text, "", "")
}

#[cfg(test)]
mod tests {
    use super::split_match;

    #[test]
    fn splits_around_the_first_match_ignoring_case() {
        assert_eq!(
            split_match("How do I deploy?", "DEPLOY"),
            ("How do I ", "deploy", "?")
        );
        assert_eq!(split_match("abcabc", "b"), ("a", "b", "cabc"));
    }

    #[test]
    fn keeps_text_without_a_match() {
        assert_eq!(split_match("Résumé tips", "x"), ("Résumé tips", "", ""));
        assert_eq!(split_match("Résumé tips", ""), ("Résumé tips", "", ""));
        assert_eq!(split_match("é", "e"), ("é", "", ""));
    }
}
