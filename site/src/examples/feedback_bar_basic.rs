use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::feedback_bar::{
    feedback_bar, feedback_bar_close, feedback_bar_helpful, feedback_bar_not_helpful,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // "" while the bar asks; then the value of the button the reader pressed.
    let answer = signal(cx, String::new);

    Ok(view! {
        <div class="w-full max-w-md">
            <div :hidden=$(answer.get() != "")>
                feedback_bar(
                    title: "Was this answer useful?",
                    // Each button carries its value, so one handler serves all three.
                    feedback_bar_helpful(
                        attrs: attributes! { @click=$(|e: Event| answer.set(e.current_target.value)) }
                    )
                    feedback_bar_not_helpful(
                        attrs: attributes! { @click=$(|e: Event| answer.set(e.current_target.value)) }
                    )
                    feedback_bar_close(
                        attrs: attributes! { @click=$(|e: Event| answer.set(e.current_target.value)) }
                    )
                )
            </div>
            // Closing the bar says nothing; an answer gets a thank-you.
            <p
                class="text-center text-sm text-muted-foreground"
                :hidden=$(if answer.get() == "" {
                    true
                } else {
                    answer.get() == "close"
                })
            >
                "Thanks for the feedback ("
                <output :value=$(answer.get())></output>
                ")."
            </p>
        </div>
    })
}
