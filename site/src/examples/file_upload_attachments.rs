use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, Signal, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    attachment::{attachment, attachments},
    button::{ButtonSize, ButtonVariant},
    file_upload::{file_upload, file_upload_content, file_upload_trigger},
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
};

/// A chip for each file in the input. `files` holds one "name<TAB>media type" line per
/// file, copied from the input whenever it changes.
#[shard]
async fn queued(cx: &Cx, files: Signal<String>) -> Result<impl View> {
    let files: Vec<(String, String)> = files
        .get()
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, media_type)| (name.to_owned(), media_type.to_owned()))
        .collect();

    Ok(view! {
        if !files.is_empty() {
            attachments(
                attrs: attributes! { class="px-3 pt-3" },
                #[key(name)]
                for (name, media_type) in &files {
                    attachment(
                        name: name,
                        media_type: media_type,
                        removable: true,
                        remove_attrs: attributes! {
                            data-file=(name)
                            @click=$(|_e: Event| {
                                // Take the file out of the input too, so it is not sent with
                                // the form. The input's change handler then updates the list.
                                raw!(
                                    "const button = ${_e}.inner.currentTarget; \
                                     const input = button.closest('[data-slot=file-upload]')\
                                         .querySelector('input[type=file]'); \
                                     const kept = new DataTransfer(); \
                                     for (const file of input.files) { \
                                         if (file.name !== button.dataset.file) { kept.items.add(file); } \
                                     } \
                                     input.files = kept.files; \
                                     input.dispatchEvent(new Event('change', { bubbles: true }));",
                                    (),
                                );
                            })
                        }
                    )
                }
            )
        }
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let files = signal(cx, String::new);
    let chips = files.clone();

    Ok(view! {
        file_upload(
            multiple: true,
            name: "attachments",
            input_attrs: attributes! {
                @change=$(|_e: Event| {
                    files.set(
                        raw!(
                            "cx.hydrate(Array.from(${_e}.inner.target.files, \
                            (file) => file.name + '\\t' + file.type).join('\\n'))",
                            String::new(),
                        ),
                    )
                })
            },
            attrs: attributes! { class="w-full rounded-xl" },
            prompt_input(
                attrs: attributes! { @submit=$(|e: Event| e.prevent_default()) },
                queued(files: chips)
                prompt_input_textarea(
                    attrs: attributes! {
                        placeholder="Pick or drop a few files, then remove one…"
                    }
                )
                prompt_input_toolbar(
                    prompt_input_tools(
                        file_upload_trigger(
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Icon,
                            attrs: attributes! { aria-label="Attach files" },
                            icon(data: iconify_icon!("lucide:paperclip"))
                        )
                    )
                    prompt_input_submit()
                )
            )
            file_upload_content("Drop files to attach them")
        )
    })
}
