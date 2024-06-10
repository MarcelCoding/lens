use axum::routing::get;
use axum::Router;

use crate::ctx::Context;
use crate::routes::image::{discover, get_image_data, get_thumbnail_data, list_images};

mod image;

pub(super) fn router() -> Router<Context> {
  Router::new()
    .route("/api/v1/image", get(list_images))
    .route("/api/v1/image/:id", get(get_image_data))
    .route("/api/v1/image/:id/thumbnail", get(get_thumbnail_data))
    .route("/api/v1/discover", get(discover))
}
