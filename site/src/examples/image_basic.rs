use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::image::image;

/// Any image URL your app serves. This one is inline, so the example needs no file.
const HARBOUR: &str = "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' \
    width='320' height='180' viewBox='0 0 320 180'%3E%3Crect width='320' height='180' fill='%23dbeafe'/%3E\
    %3Crect y='120' width='320' height='60' fill='%2393c5fd'/%3E%3Cpath d='M70 120h70l-12 \
    18H82z' fill='%23475569'/%3E%3Cpath d='M104 120V70l30 50z' fill='%23f8fafc'/%3E\
    %3Ccircle cx='260' cy='48' r='18' fill='%23fde68a'/%3E%3C/svg%3E";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        image(src: Some(HARBOUR), alt: "A sailing boat in a calm harbour at midday")
    })
}
