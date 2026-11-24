use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::copy_button::copy_button;

/// Classes for the bordered block. The header and body share its rounded corners.
const CODE_BLOCK: StaticClass = class!(
    "flex w-full min-w-0 flex-col overflow-hidden rounded-lg border border-border bg-card",
    "text-card-foreground",
);

/// A block of code with a header naming its language and a copy button.
///
/// The code keeps its whitespace and scrolls horizontally instead of wrapping. The
/// component does not highlight code. `language` labels the block and is added to the
/// `<code>` element as a `language-*` class for highlighters that look for one.
///
/// To show highlighted code, highlight it in the app and pass the markup as child
/// content: it replaces the plain text inside `<code>`, while `code` is still what the
/// copy button copies. The markup must be the same text as `code`, wrapped in elements.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     code_block(language: Some("rust"), code: "fn main() {\n    println!(\"hi\");\n}")
/// }
/// ```
#[component]
pub async fn code_block(
    cx: &Cx,
    /// The language of the code, such as `rust`.
    #[default]
    language: Option<&str>,
    /// The code.
    code: &str,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// Highlighted markup to show instead of the plain `code`.
    #[into]
    #[default]
    child: Option<Child<'_>>,
) -> Result<impl View> {
    let code_class = language.map(|language| format!("language-{language}"));

    Ok(view! {
        <div
            data-slot="code-block"
            class=(class!(CODE_BLOCK, attrs.remove("class")))
            (attrs)
        >
            code_block_header(language: language, code_block_copy(text: code))
            <pre class="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
                <code class=(code_class)>
                    match child {
                        Some(markup) => (markup),
                        None => (code),
                    }
                </code>
            </pre>
        </div>
    })
}

/// The header of a [`code_block`]: the language label on the left and actions, such
/// as a [`code_block_copy`] button, on the right.
///
/// Without a language, the label reads "Code". `attrs` are forwarded to the `<div>`.
/// Extra classes are added to its classes.
#[component]
pub async fn code_block_header(
    cx: &Cx,
    /// The language shown as the label.
    #[default]
    language: Option<&str>,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The actions on the right.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex h-10 items-center justify-between gap-2 border-b border-border",
                "bg-foreground/5 pr-0.5 pl-4 text-xs text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span class="truncate font-mono lowercase">
                (language.unwrap_or("code"))
            </span>
            <div class="flex items-center">(child)</div>
        </div>
    })
}

/// The copy button of a [`code_block`], labelled "Copy code".
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn code_block_copy(
    cx: &Cx,
    /// The code written to the clipboard.
    #[into]
    text: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    Ok(view! { copy_button(text: text, label: "Copy code", attrs: attrs) })
}
