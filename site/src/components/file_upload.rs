use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{Attributes, Child, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button};

/// A drop zone around a real `<input type="file">`.
///
/// Files the reader drops anywhere on the zone are assigned to the input, which then
/// fires a `change` event, exactly as if they had been picked. A
/// [`file_upload_trigger`] inside the zone opens the picker, and a
/// [`file_upload_content`] is the overlay shown while files are dragged over the zone.
/// The component does not upload anything: submit the input with its form, or handle
/// its `change` event, through `input_attrs`.
///
/// Dropping needs the browser runtime; the trigger and the input work without it. With
/// `multiple` off, only the first dropped file is kept. `accept` filters the picker, but
/// dropped files are not filtered, so check types on the server as usual.
///
/// `attrs` are forwarded to the outer `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     file_upload(
///         multiple: true,
///         accept: "image/*,.pdf",
///         input_attrs: attributes! { form="composer" },
///         prompt_input(...)
///         file_upload_content("Drop files to attach them")
///     )
/// }
/// ```
#[component]
pub async fn file_upload(
    cx: &Cx,
    /// Whether more than one file can be chosen.
    #[default]
    multiple: bool,
    /// The file types the picker offers, as in the `accept` attribute.
    #[default]
    accept: Option<&str>,
    /// The form field name of the input.
    #[default("files")]
    name: &str,
    /// Extra attributes for the `<input type="file">` element.
    #[default]
    input_attrs: Attributes,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The zone's content: usually a [`file_upload_trigger`] somewhere inside, and a
    /// [`file_upload_content`].
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let dragging = signal(cx, || false);

    Ok(view! {
        <div
            data-slot="file-upload"
            class=(class!("group/upload relative", attrs.remove("class")))
            :data-dragging=$(if dragging.get() { "true" } else { "false" })
            @dragover=$(|e: Event| {
                // Accepting the drag is what makes the zone a drop target.
                e.prevent_default();
                dragging.set(true);
            })
            @dragleave=$(|_e: Event| {
                // Moving between the zone's own children also fires dragleave. Related
                // targets and dropped files are outside the expression vocabulary;
                // `.inner` is the DOM event behind the runtime's event wrapper.
                if raw!(
                    "cx.hydrate(!${_e}.inner.currentTarget.contains(${_e}.inner.relatedTarget))",
                    true,
                ) {
                    dragging.set(false);
                }
            })
            @drop=$(|e: Event| {
                e.prevent_default();
                dragging.set(false);
                raw!(
                    "const zone = ${e}.inner.currentTarget; \
                     const input = zone.querySelector('input[type=file]'); \
                     const dropped = ${e}.inner.dataTransfer?.files; \
                     if (input && dropped && dropped.length > 0) { \
                         const files = new DataTransfer(); \
                         for (const file of input.multiple ? dropped : [dropped[0]]) { \
                             files.items.add(file); \
                         } \
                         input.files = files.files; \
                         input.dispatchEvent(new Event('change', { bubbles: true })); \
                     }",
                    (),
                );
            })
            (attrs)
        >
            <input
                type="file"
                hidden=""
                name=(name)
                multiple=(multiple)
                accept=(accept)
                (input_attrs)
            >
            (child)
        </div>
    })
}

/// A button that opens the file picker of the enclosing [`file_upload`].
///
/// `variant` and `size` are the built-in button's; they default to an outline button
/// with text. Give an icon-only trigger an `aria-label` in `attrs`. Opening the picker
/// needs the browser runtime. `attrs` are forwarded to the `<button>`, with extra
/// classes added to its classes.
#[component]
pub async fn file_upload_trigger(
    cx: &Cx,
    /// The button style.
    #[default(ButtonVariant::Outline)]
    variant: ButtonVariant,
    /// The button size.
    #[default(ButtonSize::Sm)]
    size: ButtonSize,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
    /// The button's text or icon.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        button(
            variant: variant,
            size: size,
            attrs: attributes! {
                type="button"
                @click=$(|_e: Event| {
                    raw!(
                        "${_e}.inner.currentTarget.closest('[data-slot=file-upload]')\
                         ?.querySelector('input[type=file]')?.click();",
                        (),
                    );
                })
                (attrs)
            },
            (child)
        )
    })
}

/// The overlay of a [`file_upload`] zone, shown only while files are dragged over it.
///
/// It covers the zone with a dashed outline and centers its content, such as "Drop
/// files here". It is hidden from assistive technology, since dragging is a pointer
/// gesture. `attrs` are forwarded to the `<div>`, with extra classes added to its
/// classes.
#[component]
pub async fn file_upload_content(
    cx: &Cx,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// What the overlay says.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            aria-hidden="true"
            class=(class!(
                "pointer-events-none absolute inset-0 z-10 hidden flex-col items-center",
                "justify-center gap-2 rounded-[inherit] border-2 border-dashed border-ring",
                "bg-background/90 text-sm text-foreground",
                "group-data-[dragging=true]/upload:flex",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
