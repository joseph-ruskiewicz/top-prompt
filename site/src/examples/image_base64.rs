use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::image::image;

/// An image as a model might return it: base64 data and a media type.
const GENERATED: &str = "PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMjAgMjAwIj48cmVjdCB3aWR0aD0iMzIwIiBoZWlnaHQ9IjIwMCIgZmlsbD0iI2U3ZTVlNCIvPjxjaXJjbGUgY3g9IjIzNiIgY3k9IjYyIiByPSIyNiIgZmlsbD0iI2ZhZmFmOSIvPjxwYXRoIGQ9Ik0wIDIwMCA5MiA5Nmw1OCA2NiA0MC0zOCAxMzAgNzZ6IiBmaWxsPSIjNzg3MTZjIi8+PHBhdGggZD0iTTAgMjAwIDcwIDE0MGw1MCA2MHoiIGZpbGw9IiM1NzUzNGUiLz48L3N2Zz4=";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        image(
            base64: Some(GENERATED),
            media_type: "image/svg+xml",
            alt: "Flat illustration of grey hills under a pale sun",
            width: Some(320),
            height: Some(200)
        )
    })
}
