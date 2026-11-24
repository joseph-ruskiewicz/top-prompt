use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::{
    branch_picker::{
        branch_picker, branch_picker_count, branch_picker_next, branch_picker_previous,
    },
    copy_button::copy_button,
    message::{message, message_actions, message_content},
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let answer = "Bake at 180°C for 25 minutes, until a skewer comes out clean.";

    Ok(view! {
        message(
            message_content((answer))
            message_actions(
                branch_picker(
                    branch_picker_previous()
                    branch_picker_count(current: 2, total: 3)
                    branch_picker_next()
                )
                copy_button(text: answer, label: "Copy message")
            )
        )
    })
}
