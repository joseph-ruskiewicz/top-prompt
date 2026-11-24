use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, page},
    view::{View, view},
};

use crate::catalog;

module_param!(pub slug: String else not_found);

/// A block alone, filling the window.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let block = catalog::block(slug(cx)?).ok_or_not_found()?;

    Ok(view! {
        <div class="flex min-h-0 flex-1 flex-col">((block.example.render)(cx))</div>
    })
}
