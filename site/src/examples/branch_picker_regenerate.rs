use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    branch_picker::{
        branch_picker, branch_picker_count, branch_picker_next, branch_picker_previous,
    },
    loader::{LoaderVariant, loader},
    message::{MessageRole, message, message_action, message_actions, message_content},
};

/// Stands in for asking the model again. Returns the versions with the new one added.
#[procedure]
async fn regenerate(cx: &Cx, mut versions: Vec<String>) -> Result<Vec<String>> {
    const ANSWERS: [&str; 3] = [
        "Try Ponte de Lima: a small town on a river, with a Roman bridge.",
        "Guimarães is an hour away, with a castle and a well-kept old town.",
        "Peneda-Gerês National Park, for waterfalls and granite villages.",
    ];
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let next = ANSWERS[versions.len() % ANSWERS.len()];
    versions.push(next.to_owned());
    Ok(versions)
}

#[shard]
async fn answer(cx: &Cx) -> Result<impl View> {
    let versions = signal(cx, || {
        vec!["Braga is a short train ride from Porto.".to_owned()]
    });
    let shown = signal(cx, || 1usize);
    let regenerating = signal(cx, || false);

    let all = versions.get();
    let total = all.len().max(1);
    let current = shown.get().clamp(1, total);
    let text = all.get(current - 1).cloned().unwrap_or_default();

    Ok(view! {
        message(
            message_content(
                if regenerating.get() {
                    loader(
                        variant: LoaderVariant::Typing,
                        label: "Writing another answer…"
                    )
                } else {
                    (text)
                }
            )
            message_actions(
                // The picker only appears once there is something to pick between.
                if total > 1 {
                    branch_picker(
                        label: "Answer versions",
                        branch_picker_previous(
                            attrs: attributes! {
                                disabled=(current == 1)
                                @click=$(|_e: Event| shown.decrement())
                            }
                        )
                        branch_picker_count(current: current, total: total)
                        branch_picker_next(
                            attrs: attributes! {
                                disabled=(current == total)
                                @click=$(|_e: Event| shown.increment())
                            }
                        )
                    )
                }
                message_action(
                    label: "Regenerate",
                    attrs: attributes! {
                        disabled=(regenerating.get())
                        @click=$(async |_e: Event| {
                            regenerating.set(true);
                            let next = regenerate(versions.get()).await;
                            // Show the newest version.
                            shown.set(next.len());
                            versions.set(next);
                            regenerating.set(false);
                        })
                    },
                    icon(data: iconify_icon!("lucide:refresh-cw"))
                )
            )
        )
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-6">
            message(
                role: MessageRole::User,
                message_content("Somewhere for a day trip from Porto?")
            )
            answer()
        </div>
    })
}
