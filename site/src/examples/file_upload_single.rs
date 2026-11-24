use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant},
    file_upload::{file_upload, file_upload_content, file_upload_trigger},
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // A browser-only URL for the chosen photo, for the preview.
    let preview = signal(cx, String::new);

    Ok(view! {
        // One image: the picker only offers images, and only the first dropped file is
        // kept. Dropped files are not filtered by `accept`, so the server checks again.
        file_upload(
            accept: Some("image/*"),
            name: "photo",
            input_attrs: attributes! {
                @change=$(|_e: Event| {
                    preview.set(
                        raw!(
                            "cx.hydrate(Array.from(${_e}.inner.target.files, \
                            (file) => URL.createObjectURL(file)).join(''))",
                            String::new(),
                        ),
                    )
                })
            },
            attrs: attributes! {
                class="flex w-full max-w-sm flex-col items-center gap-3 rounded-xl border border-dashed border-border p-6"
            },
            <img
                alt="The chosen photo"
                class="max-h-40 rounded-lg"
                :src=$(preview.get())
                :hidden=$(preview.get().is_empty())
            >
            <p class="text-sm text-muted-foreground">"A profile photo, PNG or JPEG"</p>
            file_upload_trigger(
                variant: ButtonVariant::Primary,
                size: ButtonSize::Md,
                $(if preview.get().is_empty() {
                    "Choose a photo"
                } else {
                    "Choose another"
                })
            )
            file_upload_content(<span class="font-medium">"Drop the photo here"</span>)
        )
    })
}
