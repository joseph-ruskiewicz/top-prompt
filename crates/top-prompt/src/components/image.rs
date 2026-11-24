use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{Attributes, View, class, component, view},
};

/// A generated or attached image, rounded and bordered, that scales down to fit its
/// container.
///
/// Give either `src`, a URL, or `base64` with its `media_type`, for image data a model
/// returned inline; `src` wins when both are given. `alt` is required: describe the
/// image, or pass `""` for a purely decorative one.
///
/// A pulsing placeholder sits behind the image until it loads. Pass the image's
/// `width` and `height` when you know them, so the placeholder has the right shape and
/// the page does not shift when the image arrives.
///
/// `attrs` are forwarded to the `<img>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     image(
///         base64: Some(&generated.data),
///         media_type: "image/png",
///         alt: "A lighthouse at dusk, painted in watercolor",
///         width: Some(1024),
///         height: Some(1024),
///     )
/// }
/// ```
#[component]
pub async fn image(
    cx: &Cx,
    /// The image URL.
    #[default]
    src: Option<&str>,
    /// The image data, base64-encoded, used when there is no `src`.
    #[default]
    base64: Option<&str>,
    /// The media type of `base64`, such as `image/png`.
    #[default("image/png")]
    media_type: &str,
    /// The text alternative.
    alt: &str,
    /// The image's width in pixels.
    #[default]
    width: Option<u32>,
    /// The image's height in pixels.
    #[default]
    height: Option<u32>,
    /// Extra attributes for the `<img>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let source = match (src, base64) {
        (Some(src), _) => src.to_owned(),
        (None, Some(data)) => format!("data:{media_type};base64,{data}"),
        (None, None) => String::new(),
    };
    let loaded = signal(cx, || false);

    Ok(view! {
        <span data-slot="image" class="relative block w-fit max-w-full">
            <span
                aria-hidden="true"
                class="absolute inset-0 animate-pulse rounded-xl bg-foreground/10 motion-reduce:animate-none"
                :hidden=$(loaded.get())
            ></span>
            <img
                src=(source)
                alt=(alt)
                width=(width)
                height=(height)
                loading="lazy"
                decoding="async"
                class=(class!(
                    "relative block h-auto max-w-full rounded-xl border border-border",
                    attrs.remove("class"),
                ))
                @load=$(|_e: Event| loaded.set(true))
                (attrs)
            >
        </span>
    })
}
