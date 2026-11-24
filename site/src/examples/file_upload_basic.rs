use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant},
    file_upload::{file_upload, file_upload_content, file_upload_trigger},
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // The names of the chosen files. A real app would upload them with the form.
    let chosen = signal(cx, String::new);

    Ok(view! {
        <div class="flex w-full flex-col gap-2">
            file_upload(
                multiple: true,
                input_attrs: attributes! {
                    @change=$(|_e: Event| {
                        chosen.set(
                            raw!(
                                "cx.hydrate(Array.from(${_e}.inner.target.files, (file) => file.name).join(', '))",
                                String::new(),
                            ),
                        )
                    })
                },
                attrs: attributes! { class="rounded-xl" },
                prompt_input(
                    attrs: attributes! { @submit=$(|e: Event| e.prevent_default()) },
                    prompt_input_textarea(
                        attrs: attributes! { placeholder="Describe the files…" }
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
                file_upload_content(
                    icon(
                        data: iconify_icon!("lucide:upload"),
                        attrs: attributes! { class="size-5" }
                    )
                    "Drop files to attach them"
                )
            )
            <p class="text-xs text-muted-foreground" :hidden=$(chosen.get() == "")>
                "Attached: "
                <output :value=$(chosen.get())></output>
            </p>
        </div>
    })
}
