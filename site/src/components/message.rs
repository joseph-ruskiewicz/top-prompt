use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::{
    avatar::{AvatarSize, avatar, avatar_fallback, avatar_image},
    button::{ButtonSize, ButtonVariant, button},
    floating::FloatingSide,
    tooltip::{tooltip, tooltip_content},
};

/// Who wrote a [`message`].
///
/// [`Default`] is `MessageRole::Assistant`, used when no role is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MessageRole {
    /// The person using the app. Shown as a bubble on the right.
    User,
    /// The model. Shown flat and full width on the left.
    #[default]
    Assistant,
    /// A note from the app, such as a notice that the conversation was reset. Shown
    /// centered and muted.
    System,
}

impl MessageRole {
    /// The value of the message's `data-role` attribute, which its parts style
    /// themselves from.
    fn name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    /// The grid columns of the message. The avatar column sits on the side the
    /// message is aligned to, and collapses when there is no avatar.
    fn classes(self) -> StaticClass {
        match self {
            Self::User => class!("grid-cols-[minmax(0,1fr)_auto]"),
            Self::Assistant | Self::System => class!("grid-cols-[auto_minmax(0,1fr)]"),
        }
    }
}

/// Classes for the message grid. The gap between avatar and content only appears when
/// the message has an avatar.
const MESSAGE: StaticClass = class!(
    "group/message grid w-full items-start gap-y-2",
    "has-[>[data-slot=message-avatar]]:gap-x-3",
);

/// One message in a conversation.
///
/// `role` decides the layout and defaults to `Assistant`: user messages are bubbles on
/// the right, assistant messages are flat and full width, and system messages are
/// centered notes. Add a [`message_content`] and, optionally, a [`message_avatar`] and
/// [`message_actions`] as children. The parts place themselves, so their order does not
/// matter.
///
/// The actions row is hidden until the message is hovered or holds focus. It stays
/// visible on the last message and on touch screens.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     message(
///         role: MessageRole::User,
///         message_content("How do I reverse a string in Rust?")
///     )
///     message(
///         role: MessageRole::Assistant,
///         message_avatar(fallback: "AI")
///         message_content(markdown(source: &answer))
///         message_actions(
///             copy_button(text: &answer, label: "Copy message")
///             message_action(
///                 label: "Regenerate",
///                 icon(data: iconify_icon!("lucide:refresh-cw"))
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn message(
    cx: &Cx,
    /// Who wrote the message.
    #[default]
    role: MessageRole,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The message's content, avatar, and actions.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-role=(role.name())
            class=(class!(MESSAGE, role.classes(), attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// Classes that place the avatar in its column, spanning the content and actions rows.
const AVATAR: StaticClass = class!(
    "row-span-2 row-start-1 col-start-1",
    "group-data-[role=user]/message:col-start-2",
);

/// The picture of who wrote a [`message`].
///
/// Shows the image at `src` over `fallback`, such as initials, which stays visible if
/// the image is missing or cannot load. Uses the built-in `avatar` at its small size.
///
/// `attrs` are forwarded to the avatar's outer `<span>`. Extra classes are added to its
/// classes.
///
/// ```ignore
/// view! {
///     message_avatar(src: Some("/avatars/ada.png"), fallback: "AL")
/// }
/// ```
#[component]
pub async fn message_avatar(
    cx: &Cx,
    /// Short text shown when there is no image, such as initials.
    #[into]
    #[default]
    fallback: String,
    /// The image URL.
    #[default]
    src: Option<&str>,
    /// Alternative text for the image. Leave empty when the message already names its
    /// author.
    #[into]
    #[default]
    alt: String,
    /// Extra attributes for the avatar's outer `<span>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        avatar(
            size: AvatarSize::Sm,
            attrs: attributes! {
                data-slot="message-avatar"
                class=(class!(AVATAR, attrs.remove("class")))
                (attrs)
            },
            avatar_fallback((fallback))
            if let Some(src) = src {
                avatar_image(alt: alt, attrs: attributes! { src=(src) })
            }
        )
    })
}

/// How a [`message_content`] is drawn.
///
/// [`Default`] is `MessageContentVariant::Auto`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MessageContentVariant {
    /// A bubble for user messages and flat text for the others.
    #[default]
    Auto,
    /// A tinted, rounded bubble that fits its text.
    Bubble,
    /// Plain text on the page background.
    Flat,
}

impl MessageContentVariant {
    fn classes(self) -> Option<StaticClass> {
        match self {
            Self::Auto => Some(class!(
                "group-data-[role=user]/message:rounded-2xl",
                "group-data-[role=user]/message:bg-foreground/5",
                "group-data-[role=user]/message:px-4 group-data-[role=user]/message:py-2.5",
            )),
            // A user message's own placement is more specific and wins over
            // `justify-self-start`.
            Self::Bubble => Some(class!(
                "max-w-[85%] justify-self-start rounded-2xl bg-foreground/5 px-4 py-2.5",
            )),
            Self::Flat => None,
        }
    }
}

/// Classes for the content column. User content hugs the right edge and leaves room on
/// the left; system notes are centered and muted.
const CONTENT: StaticClass = class!(
    "col-start-2 flex min-w-0 flex-col gap-3 text-sm leading-relaxed",
    "break-words text-foreground",
    "group-data-[role=user]/message:col-start-1",
    "group-data-[role=user]/message:max-w-[85%]",
    "group-data-[role=user]/message:justify-self-end",
    "group-data-[role=user]/message:whitespace-pre-wrap",
    "group-data-[role=system]/message:justify-self-center",
    "group-data-[role=system]/message:text-xs",
    "group-data-[role=system]/message:text-muted-foreground",
);

/// The body of a [`message`]: text, markdown, reasoning, tool calls, and so on.
///
/// `variant` defaults to `Auto`, which draws user messages as a bubble and the others
/// flat. User message text keeps its line breaks.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn message_content(
    cx: &Cx,
    /// How the content is drawn.
    #[default]
    variant: MessageContentVariant,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The message body.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!(CONTENT, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// Classes for the actions row. It is hidden until the message is hovered or holds
/// focus, and always visible on the last message and on touch screens.
const ACTIONS: StaticClass = class!(
    "col-start-2 -mt-1 flex items-center gap-0.5 text-muted-foreground",
    "opacity-0 transition-opacity duration-150",
    "group-hover/message:opacity-100 group-focus-within/message:opacity-100",
    "group-last/message:opacity-100 pointer-coarse:opacity-100",
    "group-data-[role=user]/message:col-start-1",
    "group-data-[role=user]/message:justify-self-end",
);

/// A toolbar of actions under a [`message`], such as copy, regenerate, and feedback.
///
/// Add [`message_action`] buttons, a `copy_button`, or a `branch_picker` as children.
/// `label` names the toolbar for assistive technology.
///
/// `attrs` are forwarded to the `<div>`. Extra classes are added to its classes.
#[component]
pub async fn message_actions(
    cx: &Cx,
    /// The accessible name of the toolbar.
    #[into]
    #[default(String::from("Message actions"))]
    label: String,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The action buttons.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="toolbar"
            aria-label=(label)
            class=(class!(ACTIONS, attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A ghost icon button in [`message_actions`], with a tooltip.
///
/// Pass the icon as children. `label` is shown in the tooltip and is the button's
/// accessible name, so the icon itself needs no label. Attach behavior through `attrs`,
/// such as an `@click` handler or `type="submit"` with a `form`.
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     message_action(
///         label: "Good response",
///         attrs: attributes! { @click=$(|_e: Event| liked.set(true)) },
///         icon(data: iconify_icon!("lucide:thumbs-up"))
///     )
/// }
/// ```
#[component]
pub async fn message_action(
    cx: &Cx,
    /// The tooltip and accessible name of the button.
    #[into]
    label: String,
    /// Where the tooltip appears.
    #[default(FloatingSide::Top)]
    side: FloatingSide,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
    /// The icon.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        tooltip(
            button(
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Icon,
                attrs: attributes! {
                    if !attrs.contains_key("type") {
                        type="button"
                    }
                    aria-label=(&label)
                    (attrs)
                },
                (child)
            )
            tooltip_content(side: side, (label))
        )
    })
}
