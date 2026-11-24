use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{Attributes, Child, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// A wrapping row of [`attachment`] chips, for files queued in a prompt input or sent
/// with a message.
///
/// `attrs` are forwarded to the `<ul>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     attachments(
///         attachment(name: "report.pdf", media_type: "application/pdf", removable: true)
///         attachment(
///             name: "chart.png",
///             media_type: "image/png",
///             preview_src: Some("/uploads/chart.png"),
///         )
///     )
/// }
/// ```
#[component]
pub async fn attachments(
    cx: &Cx,
    /// Extra attributes for the `<ul>` element.
    #[default]
    mut attrs: Attributes,
    /// The [`attachment`] chips.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul
            aria-label="Attachments"
            class=(class!("flex flex-wrap gap-2", attrs.remove("class")))
            (attrs)
        >
            (child)
        </ul>
    })
}

/// The icon for a file of the given media type.
fn file_icon(media_type: &str) -> IconData {
    let (kind, subtype) = media_type.split_once('/').unwrap_or((media_type, ""));
    match (kind, subtype) {
        ("image", _) => iconify_icon!("lucide:file-image"),
        ("audio", _) => iconify_icon!("lucide:file-audio"),
        ("video", _) => iconify_icon!("lucide:file-video"),
        ("text", "csv") | (_, "vnd.ms-excel") => iconify_icon!("lucide:file-spreadsheet"),
        ("text", _) | (_, "json" | "pdf" | "xml") => iconify_icon!("lucide:file-text"),
        (_, "zip" | "gzip" | "x-tar" | "x-7z-compressed") => iconify_icon!("lucide:file-archive"),
        _ => iconify_icon!("lucide:file"),
    }
}

/// A file chip: a thumbnail or file-type icon, the file name, and an optional remove
/// button.
///
/// Images with a `preview_src` show a thumbnail; other files show an icon chosen from
/// `media_type`, such as `application/pdf`. Long names are truncated, with the full
/// name in a tooltip. Set `removable` to add an [`attachment_remove`] button: clicking
/// it hides the chip. The chip only displays files, so tell your app the file is gone
/// through `remove_attrs`, such as an `@click` handler that calls a procedure or
/// updates a signal; it runs before the chip hides. Uploading files is the app's job.
///
/// In a loop, key the iterations (`#[key(file.id)]`) so each chip keeps its own state.
///
/// `attrs` are forwarded to the `<li>`. Extra classes are added to its classes.
#[component]
pub async fn attachment(
    cx: &Cx,
    /// The file name.
    name: &str,
    /// The file's media type, such as `image/png`.
    #[default]
    media_type: &str,
    /// The URL of an image thumbnail.
    #[default]
    preview_src: Option<&str>,
    /// Whether to show a remove button.
    #[default]
    removable: bool,
    /// Extra attributes for the remove `<button>` element.
    #[default]
    remove_attrs: Attributes,
    /// Extra attributes for the `<li>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let is_image = media_type.starts_with("image/");
    // Set when the reader removes the chip, so it stays hidden across later renders.
    let removed = signal(cx, || false);

    Ok(view! {
        <li
            title=(name)
            :hidden=$(removed.get())
            class=(class!(
                "flex max-w-64 items-center gap-2 rounded-lg border border-border",
                "bg-background p-1 text-sm text-foreground",
                "pr-3" if !removable,
                attrs.remove("class"),
            ))
            (attrs)
        >
            match preview_src {
                Some(src) if is_image => {
                    <img
                        src=(src)
                        alt=""
                        class="size-9 shrink-0 rounded-md object-cover"
                    >
                }
                _ => {
                    <span
                        class="flex size-9 shrink-0 items-center justify-center rounded-md bg-foreground/5 text-muted-foreground"
                    >
                        icon(data: file_icon(media_type))
                    </span>
                }
            }
            <span class="min-w-0 flex-1 truncate">(name)</span>
            if removable {
                // The click bubbles up from the button, after any handler the app put
                // on it through `remove_attrs`.
                <span class="contents" @click=$(|_e: Event| removed.set(true))>
                    attachment_remove(name: name, attrs: remove_attrs)
                </span>
            }
        </li>
    })
}

/// The remove button of an [`attachment`]: a ghost icon button labelled "Remove" and
/// the file name.
///
/// Attach the remove action through `attrs`. They are forwarded to the `<button>`, with
/// extra classes added to its classes.
#[component]
pub async fn attachment_remove(
    cx: &Cx,
    /// The file name, used in the accessible label.
    name: &str,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        button(
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Icon,
            attrs: attributes! {
                if !attrs.contains_key("type") {
                    type="button"
                }
                aria-label=(format!("Remove {name}"))
                (attrs)
            },
            icon(data: iconify_icon!("lucide:x"))
        )
    })
}
