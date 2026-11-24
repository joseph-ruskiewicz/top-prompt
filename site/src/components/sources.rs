use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, element_id, view},
};

use super::{
    badge::{BadgeVariant, badge_variants},
    hover_card::{hover_card, hover_card_content},
};

/// Classes that animate the list height when it opens or closes, as the built-in
/// accordion does.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0",
    "[&::details-content]:overflow-hidden",
    "[&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete]",
    "[&[open]::details-content]:h-auto",
);

/// A collapsed "Used N sources" list of the links a response drew on.
///
/// Uses a native `<details>` element and starts closed. `count` is shown in the
/// trigger. Pass a [`sources_content`] holding one [`source`] per link as child
/// content. With the browser runtime, the reader's choice to open or close the list
/// survives the page or a live region rendering it again.
///
/// `attrs` are forwarded to the `<details>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     sources(
///         count: links.len(),
///         sources_content(
///             for link in &links {
///                 source(href: &link.url, title: &link.title)
///             }
///         )
///     )
/// }
/// ```
#[component]
pub async fn sources(
    cx: &Cx,
    /// The number of sources, shown in the trigger.
    count: usize,
    /// Extra attributes for the `<details>` element.
    #[default]
    mut attrs: Attributes,
    /// A [`sources_content`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let id = element_id(cx);
    let open = signal(cx, || false);

    Ok(view! {
        <details
            id=(&id)
            data-slot="sources"
            class=(class!("group/sources w-full", ANIMATION, attrs.remove("class")))
            :open=$(open.get())
            @toggle=$(|_e: Event| {
                // The details element's open state is outside the expression vocabulary.
                open.set(
                    raw!(
                        "cx.hydrate(document.getElementById(${id}.toString()).open)",
                        false,
                    ),
                )
            })
            (attrs)
        >
            sources_trigger(count: count)
            (child)
        </details>
    })
}

/// The clickable summary of a [`sources`] list, reading "Used N sources".
///
/// [`sources`] renders one from its `count`; use this directly only in a custom
/// `<details>`. `attrs` are forwarded to the `<summary>`. Extra classes are added to
/// its classes.
#[component]
pub async fn sources_trigger(
    cx: &Cx,
    /// The number of sources.
    count: usize,
    /// Extra attributes for the `<summary>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <summary
            class=(class!(
                "flex w-fit cursor-pointer list-none items-center gap-2 rounded-md text-xs",
                "font-medium text-muted-foreground outline-none transition-colors",
                "hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                "focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                "[&::-webkit-details-marker]:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            "Used "
            (count)
            if count == 1 {
                " source"
            } else {
                " sources"
            }
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="transition-transform duration-200 ease-out group-open/sources:rotate-180"
                }
            )
        </summary>
    })
}

/// The list inside a [`sources`] section. Pass one [`source`] per link as children.
///
/// `attrs` are forwarded to the `<ul>`. Extra classes are added to its classes.
#[component]
pub async fn sources_content(
    cx: &Cx,
    /// Extra attributes for the `<ul>` element.
    #[default]
    mut attrs: Attributes,
    /// The [`source`] links.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul class=(class!("flex flex-col gap-2 pt-3", attrs.remove("class"))) (attrs)>
            (child)
        </ul>
    })
}

/// One link in a [`sources_content`] list. Opens in a new tab.
///
/// `title` is the link text; it falls back to `href`. The URL is rendered as given, so
/// check that URLs from a model or tool use `http` or `https` before passing them. `attrs` are forwarded to the
/// `<a>`. Extra classes are added to its classes.
#[component]
pub async fn source(
    cx: &Cx,
    /// The URL of the source.
    href: &str,
    /// The link text.
    #[default]
    title: &str,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <li class="flex min-w-0">
            <a
                href=(href)
                target="_blank"
                rel="noopener noreferrer"
                class=(class!(
                    "inline-flex min-w-0 items-center gap-2 rounded-sm text-sm",
                    "text-muted-foreground outline-none transition-colors",
                    "hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                    "focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                icon(
                    data: iconify_icon!("lucide:link"),
                    attrs: attributes! { class="shrink-0" }
                )
                <span class="truncate">
                    (if title.is_empty() { href } else { title })
                </span>
            </a>
        </li>
    })
}

/// A numbered citation marker linking to a source, with a hover card previewing it.
///
/// Place it inline after the claim it supports. `label` is the marker's text, usually
/// the source's number. The hover card shows `title`, falling back to `href`, and the
/// URL; it also opens when the marker has keyboard focus. As with [`source`], check the
/// scheme of URLs that come from a model or tool.
///
/// `attrs` are forwarded to the `<a>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     <p>
///         "Live regions stream over the HTTP response."
///         inline_citation(label: "1", href: "https://docs.rs/topcoat", title: "Topcoat docs")
///     </p>
/// }
/// ```
#[component]
pub async fn inline_citation(
    cx: &Cx,
    /// The marker text, such as `1`.
    #[into]
    label: String,
    /// The URL of the source.
    href: &str,
    /// The title of the source.
    #[default]
    title: &str,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let name = if title.is_empty() { href } else { title };

    Ok(view! {
        hover_card(
            attrs: attributes! { class="mx-0.5 align-baseline" },
            <a
                href=(href)
                target="_blank"
                rel="noopener noreferrer"
                aria-label=(format!("Source {label}: {name}"))
                class=(class!(
                    badge_variants(BadgeVariant::Secondary),
                    "tabular-nums no-underline outline-none transition-colors",
                    "hover:bg-foreground/10 focus-visible:ring-2 focus-visible:ring-ring",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                (label)
            </a>
            hover_card_content(
                <span class="text-sm font-medium">(name)</span>
                <span class="truncate text-xs text-muted-foreground">(href)</span>
            )
        )
    })
}

/// A compact link to a source, for listing sources inline under an answer. Hovering or
/// focusing it shows a card with the source's title and description.
///
/// The chip shows `label`, or the link's host when no label is given. With
/// `show_favicon`, it starts with the site's `/favicon.ico`, which the reader's browser
/// loads from that site. As with [`source`], check the URL's scheme before rendering
/// links a model produced.
///
/// `attrs` are forwarded to the `<a>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     <div class="flex flex-wrap gap-2">
///         source_chip(
///             href: "https://doc.rust-lang.org/book/",
///             title: "The Rust Programming Language",
///             description: "An introductory book about Rust.",
///             show_favicon: true,
///         )
///     </div>
/// }
/// ```
#[component]
pub async fn source_chip(
    cx: &Cx,
    /// The URL of the source.
    href: &str,
    /// The chip's text; defaults to the link's host.
    #[default]
    label: &str,
    /// The title shown in the hover card; defaults to the URL.
    #[default]
    title: &str,
    /// A sentence about the source, shown in the hover card.
    #[default]
    description: &str,
    /// Whether to show the site's favicon in the chip.
    #[default]
    show_favicon: bool,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let host = host(href);
    let label = if label.is_empty() { host } else { label };
    let title = if title.is_empty() { href } else { title };
    let favicon = (show_favicon && !host.is_empty()).then(|| {
        let scheme = if href.starts_with("http://") {
            "http"
        } else {
            "https"
        };
        format!("{scheme}://{host}/favicon.ico")
    });

    Ok(view! {
        hover_card(
            <a
                href=(href)
                target="_blank"
                rel="noopener noreferrer"
                class=(class!(
                    "inline-flex h-7 max-w-48 items-center gap-1.5 rounded-full",
                    "bg-foreground/5 px-2.5 text-xs text-muted-foreground no-underline",
                    "outline-none transition-colors hover:bg-foreground/10",
                    "hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                if let Some(favicon) = favicon {
                    <img
                        src=(favicon)
                        alt=""
                        width="14"
                        height="14"
                        class="size-3.5 shrink-0 rounded-sm"
                    >
                }
                <span class="truncate">(label)</span>
            </a>
            hover_card_content(
                <span class="text-sm font-medium">(title)</span>
                if !description.is_empty() {
                    <span class="line-clamp-3 text-xs text-muted-foreground">
                        (description)
                    </span>
                }
                <span class="truncate text-xs text-muted-foreground">(href)</span>
            )
        )
    })
}

/// The host of an absolute URL, without a port or credentials, or `""` for anything
/// else.
fn host(href: &str) -> &str {
    let Some((_, rest)) = href.split_once("://") else {
        return "";
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or("");
    authority.split(':').next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::host;

    #[test]
    fn host_of_a_url() {
        assert_eq!(host("https://docs.rs/topcoat/latest"), "docs.rs");
        assert_eq!(host("http://user@example.com:8080/a?b"), "example.com");
        assert_eq!(host("https://example.com"), "example.com");
        assert_eq!(host("/relative/path"), "");
    }
}
