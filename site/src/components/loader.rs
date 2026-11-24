use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Length, StaticClass, View, class, component, view},
};

use super::{spinner::spinner, text_shimmer::text_shimmer};

/// The animation of a [`loader`].
///
/// [`Default`] is `LoaderVariant::Dots`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LoaderVariant {
    /// Three dots bouncing one after another.
    #[default]
    Dots,
    /// A single pulsing dot.
    Pulse,
    /// The label itself, pulsing.
    Text,
    /// A spinning circle: the built-in `spinner`.
    Circular,
    /// Three dots fading in turn, like a typing indicator.
    Typing,
    /// Five bars rising and falling in a wave.
    Wave,
    /// Three bars growing in turn.
    Bars,
    /// The label with a highlight sweeping across it: a [`text_shimmer`].
    TextShimmer,
}

/// The size of a [`loader`].
///
/// [`Default`] is `LoaderSize::Md`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LoaderSize {
    /// For inline use next to small text.
    Sm,
    /// For a message body.
    #[default]
    Md,
    /// For an empty panel.
    Lg,
}

impl LoaderSize {
    /// The font size, which scales the label and the em-sized parts.
    fn text(self) -> StaticClass {
        match self {
            Self::Sm => class!("text-xs"),
            Self::Md => class!("text-sm"),
            Self::Lg => class!("text-base"),
        }
    }

    /// The size of one dot.
    fn dot(self) -> StaticClass {
        match self {
            Self::Sm => class!("size-1"),
            Self::Md => class!("size-1.5"),
            Self::Lg => class!("size-2"),
        }
    }

    /// The height of the bar variants.
    fn bars(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-3"),
            Self::Md => class!("h-4"),
            Self::Lg => class!("h-5"),
        }
    }
}

/// Keyframes Tailwind does not ship: a fade for `Typing` and a vertical scale for
/// `Wave` and `Bars`. Every loader on a page shares the names, and repeating the rules
/// is harmless.
const KEYFRAMES: &str = "@keyframes topcoat-prompt-fade { 0%, 100% { opacity: 0.25; } \
                         50% { opacity: 1; } } \
                         @keyframes topcoat-prompt-scale { 0%, 100% { transform: scaleY(0.4); } \
                         50% { transform: scaleY(1); } }";

/// Classes shared by every loader. `relative` contains the screen reader label, which
/// would otherwise stretch the page's scroll area.
const BASE: StaticClass = class!("relative inline-flex items-center gap-1 text-muted-foreground");

/// Classes for one bouncing dot. Each dot sets its own delay.
const DOT: StaticClass =
    class!("rounded-full bg-current animate-bounce motion-reduce:animate-none");

/// Classes for one fading dot of `Typing`.
const FADING_DOT: StaticClass = class!(
    "rounded-full bg-current animate-[topcoat-prompt-fade_1.2s_ease-in-out_infinite]",
    "motion-reduce:animate-none",
);

/// Classes for one bar of `Wave` and `Bars`. Each bar sets its own animation and delay.
const BAR: StaticClass =
    class!("h-full rounded-full bg-current origin-center motion-reduce:animate-none",);

/// An indicator for the wait before an assistant's first token arrives.
///
/// `variant` chooses the animation and defaults to `Dots`; `size` scales it. `label`
/// describes the wait to assistive technology; the `Text` and `TextShimmer` variants
/// also show it. The root has `role="status"`, so a change of label is announced.
/// Users who prefer reduced motion see the indicators stand still.
///
/// `attrs` are forwarded to the outer `<span>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     message(
///         role: MessageRole::Assistant,
///         message_content(loader(variant: LoaderVariant::TextShimmer, label: "Searching…"))
///     )
/// }
/// ```
#[component]
pub async fn loader(
    cx: &Cx,
    /// The animation to show.
    #[default]
    variant: LoaderVariant,
    /// The size of the indicator.
    #[default]
    size: LoaderSize,
    /// The description of the wait.
    #[into]
    #[default(String::from("Thinking…"))]
    label: String,
    /// Extra attributes for the outer `<span>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <span
            role="status"
            data-slot="loader"
            class=(class!(BASE, size.text(), attrs.remove("class")))
            (attrs)
        >
            match variant {
                LoaderVariant::Dots => {
                    <span
                        class="inline-flex h-[1em] items-end gap-1 pb-0.5"
                        aria-hidden="true"
                    >
                        <span
                            class=(class!(DOT, size.dot(), "[animation-delay:-300ms]"))
                        ></span>
                        <span
                            class=(class!(DOT, size.dot(), "[animation-delay:-150ms]"))
                        ></span>
                        <span class=(class!(DOT, size.dot()))></span>
                    </span>
                    <span class="sr-only">(label)</span>
                }
                LoaderVariant::Pulse => {
                    <span
                        class=(class!(
                            "rounded-full bg-foreground animate-pulse motion-reduce:animate-none",
                            match size {
                                LoaderSize::Sm => "size-2",
                                LoaderSize::Md => "size-2.5",
                                LoaderSize::Lg => "size-3",
                            },
                        ))
                        aria-hidden="true"
                    ></span>
                    <span class="sr-only">(label)</span>
                }
                LoaderVariant::Text => {
                    <span class="animate-pulse motion-reduce:animate-none">
                        (label)
                    </span>
                }
                LoaderVariant::Circular => {
                    spinner(
                        label: label,
                        size: match size {
                            LoaderSize::Sm => Length::em(1.0),
                            LoaderSize::Md => Length::em(1.25),
                            LoaderSize::Lg => Length::em(1.5),
                        }
                    )
                }
                LoaderVariant::Typing => {
                    <style>(KEYFRAMES)</style>
                    <span
                        class="inline-flex h-[1em] items-center gap-1"
                        aria-hidden="true"
                    >
                        <span class=(class!(FADING_DOT, size.dot()))></span>
                        <span
                            class=(class!(FADING_DOT, size.dot(), "[animation-delay:200ms]"))
                        ></span>
                        <span
                            class=(class!(FADING_DOT, size.dot(), "[animation-delay:400ms]"))
                        ></span>
                    </span>
                    <span class="sr-only">(label)</span>
                }
                LoaderVariant::Wave => {
                    <style>(KEYFRAMES)</style>
                    <span
                        class=(class!("inline-flex items-center gap-0.5", size.bars()))
                        aria-hidden="true"
                    >
                        for delay in [0, 100, 200, 300, 400] {
                            <span
                                class=(class!(
                                    BAR,
                                    "w-0.5 animate-[topcoat-prompt-scale_1s_ease-in-out_infinite]",
                                ))
                                style=(format!("animation-delay: {delay}ms"))
                            ></span>
                        }
                    </span>
                    <span class="sr-only">(label)</span>
                }
                LoaderVariant::Bars => {
                    <style>(KEYFRAMES)</style>
                    <span
                        class=(class!("inline-flex items-center gap-1", size.bars()))
                        aria-hidden="true"
                    >
                        for delay in [-240, -120, 0] {
                            <span
                                class=(class!(
                                    BAR,
                                    "w-1 animate-[topcoat-prompt-scale_0.9s_ease-in-out_infinite]",
                                ))
                                style=(format!("animation-delay: {delay}ms"))
                            ></span>
                        }
                    </span>
                    <span class="sr-only">(label)</span>
                }
                LoaderVariant::TextShimmer => {
                    text_shimmer((label))
                }
            }
        </span>
    })
}
