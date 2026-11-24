use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    attachment::{attachment, attachments},
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
    },
};

/// Stands in for deleting an upload on the server. Returns the files that are left.
#[procedure]
async fn delete_upload(
    cx: &Cx,
    files: Vec<(String, String)>,
    name: String,
) -> Result<Vec<(String, String)>> {
    Ok(files
        .into_iter()
        .filter(|(file, _)| *file != name)
        .collect())
}

/// The composer re-renders on the server whenever its list of files changes.
#[shard]
async fn composer(cx: &Cx) -> Result<impl View> {
    // (name, media type) of each file already uploaded for this draft.
    let files = signal(cx, || {
        vec![
            ("itinerary.pdf".to_owned(), "application/pdf".to_owned()),
            (
                "budget.xlsx".to_owned(),
                "application/vnd.ms-excel".to_owned(),
            ),
            ("tickets.png".to_owned(), "image/png".to_owned()),
        ]
    });
    let current = files.get();
    let count = match current.len() {
        0 => "No files attached".to_owned(),
        1 => "1 file attached".to_owned(),
        n => format!("{n} files attached"),
    };

    Ok(view! {
        prompt_input(
            attrs: attributes! { class="w-full" },
            if !current.is_empty() {
                attachments(
                    attrs: attributes! { class="px-3 pt-3" },
                    #[key(name)]
                    for (name, media_type) in &current {
                        let name = name.clone();
                        attachment(
                            name: &name,
                            media_type: media_type,
                            removable: true,
                            remove_attrs: attributes! {
                                @click=$(async |_e: Event| files.set(
                                        delete_upload(files.get(), name).await,
                                    ))
                            }
                        )
                    }
                )
            }
            prompt_input_textarea(
                attrs: attributes! { placeholder="Ask about these files…" }
            )
            prompt_input_toolbar(
                <span class="px-2 text-xs text-muted-foreground">(count)</span>
                prompt_input_submit()
            )
        )
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { composer() })
}
