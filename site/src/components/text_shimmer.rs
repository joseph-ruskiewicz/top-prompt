use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The keyframes of the sweep. Tailwind ships no shimmer animation, so the component
/// brings its own; every shimmer on a page shares the name, and repeating the rule is
/// harmless.
const KEYFRAMES: &str = "@keyframes top-prompt-shimmer { from { background-position: 100% 0; } \
                         to { background-position: 0% 0; } }";

/// Classes that clip the gradient to the text. With reduced motion the highlight
/// stands still.
const SHIMMER: StaticClass = class!(
    "inline-block bg-size-[250%_100%] bg-clip-text bg-no-repeat text-transparent",
    "animate-[top-prompt-shimmer_linear_infinite] motion-reduce:animate-none",
);

/// Muted text with a highlight sweeping across it, for "Thinking", "Searching the
/// web" and other in-progress labels.
///
/// `duration_ms` is how long one sweep takes and `spread` is the width of the highlight
/// in pixels. Users who prefer reduced motion see the highlight stand still. The colors
/// come from the theme's `foreground` and `muted-foreground`.
///
/// The text is not a live region: put it in a `role="status"` element, such as a
/// [`loader`](super::loader::loader), when changes should be announced. `attrs` are
/// forwarded to the `<span>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     text_shimmer(duration_ms: 1500, "Searching the docs…")
/// }
/// ```
#[component]
pub async fn text_shimmer(
    cx: &Cx,
    /// How long one sweep of the highlight takes, in milliseconds.
    #[default(2000)]
    duration_ms: u32,
    /// The width of the highlight, in pixels.
    #[default(40)]
    spread: u32,
    /// Extra attributes for the `<span>` element.
    #[default]
    mut attrs: Attributes,
    /// The text.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    // The first layer is the moving highlight, the second the muted base color under
    // it. Both use theme colors.
    let style = format!(
        "background-image: linear-gradient(90deg, transparent calc(50% - {spread}px), \
         var(--foreground) 50%, transparent calc(50% + {spread}px)), \
         linear-gradient(var(--muted-foreground), var(--muted-foreground)); \
         animation-duration: {duration_ms}ms"
    );

    Ok(view! {
        <style>(KEYFRAMES)</style>
        <span
            data-slot="text-shimmer"
            class=(class!(SHIMMER, attrs.remove("class")))
            style=(style)
            (attrs)
        >
            (child)
        </span>
    })
}
