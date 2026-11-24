use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::ButtonSize,
    prompt_input::{
        prompt_input, prompt_input_button, prompt_input_submit, prompt_input_textarea,
        prompt_input_toolbar, prompt_input_tools,
    },
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);
    let searching = signal(cx, || false);

    Ok(view! {
        <div class="flex h-full w-full flex-col items-center justify-end p-6">
            prompt_input(
                attrs: attributes! {
                    class="max-w-2xl"
                    @submit=$(|e: Event| {
                        e.prevent_default();
                        draft.set("".to_owned());
                    })
                },
                prompt_input_textarea(
                    attrs: attributes! {
                        placeholder="Ask anything"
                        :value=$(draft.get())
                        @input=$(|e: Event| draft.set(e.target.value))
                    }
                )
                prompt_input_toolbar(
                    prompt_input_tools(
                        prompt_input_button(
                            attrs: attributes! { aria-label="Attach files" },
                            icon(data: iconify_icon!("lucide:plus"))
                        )
                        // A toggle: pressed while web search is on.
                        prompt_input_button(
                            size: ButtonSize::Sm,
                            attrs: attributes! {
                                class="aria-pressed:bg-foreground/10"
                                :aria-pressed=$(if searching.get() {
                                    "true"
                                } else {
                                    "false"
                                })
                                @click=$(|_e: Event| searching.toggle())
                            },
                            icon(data: iconify_icon!("lucide:globe"))
                            "Search"
                        )
                        prompt_input_button(
                            attrs: attributes! { aria-label="More tools" },
                            icon(data: iconify_icon!("lucide:ellipsis"))
                        )
                    )
                    <div class="flex items-center gap-1">
                        prompt_input_button(
                            attrs: attributes! { aria-label="Dictate" },
                            icon(data: iconify_icon!("lucide:mic"))
                        )
                        prompt_input_submit(disabled: $(draft.get().trim().is_empty()))
                    </div>
                )
            )
        </div>
    })
}
