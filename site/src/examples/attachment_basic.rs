use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::attachment::{attachment, attachments};

/// A small landscape, inlined so the example needs no image file.
const THUMBNAIL: &str = "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' \
    viewBox='0 0 40 40'%3E%3Crect width='40' height='40' fill='%23d6d3d1'/%3E%3Ccircle \
    cx='29' cy='11' r='5' fill='%23fafaf9'/%3E%3Cpath d='M0 40 15 21l9 11 6-6 10 14z' \
    fill='%2378716c'/%3E%3C/svg%3E";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        attachments(
            attachment(
                name: "floor-plan.png",
                media_type: "image/png",
                preview_src: Some(THUMBNAIL),
                removable: true
            )
            attachment(
                name: "lease-agreement-2026.pdf",
                media_type: "application/pdf",
                removable: true
            )
            attachment(name: "measurements.csv", media_type: "text/csv")
            attachment(name: "photos.zip", media_type: "application/zip")
        )
    })
}
