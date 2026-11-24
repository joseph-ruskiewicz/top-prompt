use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// The kind of a [`system_message`].
///
/// [`Default`] is `SystemMessageVariant::Action`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SystemMessageVariant {
    /// Neutral information, often with an action such as "Upgrade" or "Retry".
    #[default]
    Action,
    /// Something the reader should know before going on. The theme has no warning
    /// color, so warnings use the foreground color and a warning icon.
    Warning,
    /// Something failed. The icon and border use the theme's `destructive` color.
    Error,
}

impl SystemMessageVariant {
    /// The icon shown when the message does not set its own.
    fn icon(self) -> IconData {
        match self {
            Self::Action => iconify_icon!("lucide:info"),
            Self::Warning => iconify_icon!("lucide:triangle-alert"),
            Self::Error => iconify_icon!("lucide:circle-x"),
        }
    }

    /// The text and border colors.
    fn classes(self, fill: bool) -> StaticClass {
        match (self, fill) {
            (Self::Action, false) => class!("border-border text-muted-foreground"),
            (Self::Action, true) => class!("border-transparent bg-foreground/5 text-foreground"),
            (Self::Warning, false) => class!("border-foreground/25 text-foreground"),
            (Self::Warning, true) => {
                class!("border-foreground/15 bg-foreground/10 text-foreground")
            }
            // The message stays in the foreground color so it is readable on the tint;
            // the icon and border carry the destructive color.
            (Self::Error, false) => {
                class!("border-destructive/50 text-foreground [&>svg]:text-destructive")
            }
            (Self::Error, true) => class!(
                "border-destructive/50 bg-destructive/10 text-foreground",
                "[&>svg]:text-destructive",
            ),
        }
    }
}

/// A notice inside a conversation: a limit reached, a model switched, a request that
/// failed.
///
/// `variant` sets the tone and its icon; pass `leading_icon` to use another one. `fill` gives
/// the notice a tinted background instead of only a border. Put a
/// [`system_message_cta`] in the child content for an action. Errors have
/// `role="alert"`; the others have `role="status"`.
///
/// `attrs` are forwarded to the outer `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     system_message(
///         variant: SystemMessageVariant::Warning,
///         "You have 3 messages left today."
///         system_message_cta(attrs: attributes! { href="/plans" }, "See plans")
///     )
/// }
/// ```
#[component]
pub async fn system_message(
    cx: &Cx,
    /// The tone of the notice.
    #[default]
    variant: SystemMessageVariant,
    /// Whether the notice has a tinted background.
    #[default]
    fill: bool,
    /// An icon to show instead of the variant's.
    #[default]
    leading_icon: Option<IconData>,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The text, and optionally a [`system_message_cta`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let data = leading_icon.unwrap_or(variant.icon());

    Ok(view! {
        <div
            role=(if variant == SystemMessageVariant::Error {
                "alert"
            } else {
                "status"
            })
            data-slot="system-message"
            data-variant=(format!("{variant:?}").to_lowercase())
            class=(class!(
                "flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-sm",
                variant.classes(fill),
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(data: data, attrs: attributes! { class="shrink-0" })
            <div
                class="flex min-w-0 flex-1 flex-wrap items-center justify-between gap-x-3 gap-y-1"
            >
                (child)
            </div>
        </div>
    })
}

/// An action in a [`system_message`]: a small outline button, or a link when `attrs`
/// has an `href`.
///
/// `attrs` are forwarded to the `<button>` or `<a>`, with extra classes added to its
/// classes.
#[component]
pub async fn system_message_cta(
    cx: &Cx,
    /// Extra attributes for the `<button>` or `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The action's label.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let link = attrs.contains_key("href");

    Ok(view! {
        if link {
            <a
                class=(class!(
                    "shrink-0 text-sm font-medium text-foreground underline underline-offset-4",
                    "rounded-sm outline-none hover:no-underline focus-visible:ring-2",
                    "focus-visible:ring-ring",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                (child)
            </a>
        } else {
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    if !attrs.contains_key("type") {
                        type="button"
                    }
                    class="shrink-0"
                    (attrs)
                },
                (child)
            )
        }
    })
}
