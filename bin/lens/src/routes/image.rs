use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Html;
use axum::Json;
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ctx::Context;

#[derive(Deserialize)]
pub(super) struct ListQuery {
  #[serde(with = "time::serde::rfc3339")]
  from: OffsetDateTime,
  #[serde(with = "time::serde::rfc3339::option")]
  to: Option<OffsetDateTime>,
  limit: Option<u64>,
}

pub(super) async fn list_images(
  State(ctx): State<Context>,
  Query(query): Query<ListQuery>,
) -> Json<Vec<entity::image::Model>> {
  Json(
    ctx
      .image_service
      .list(query.from, query.to, query.limit.unwrap_or(100))
      .await
      .unwrap(),
  )
}

pub(super) async fn get_image_data(
  State(ctx): State<Context>,
  Path(id): Path<Uuid>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
  let path = ctx
    .image_service
    .get_image_path(id)
    .await
    .unwrap()
    .ok_or(StatusCode::NOT_FOUND)?;

  let guess = mime_guess::from_path(&path);
  let mime = guess
    .first_raw()
    .map(HeaderValue::from_static)
    .unwrap_or_else(|| HeaderValue::from_str(mime::APPLICATION_OCTET_STREAM.as_ref()).unwrap());

  let mut headers = HeaderMap::new();
  headers.insert(CONTENT_TYPE, mime);
  headers.insert(
    CONTENT_DISPOSITION,
    HeaderValue::from_str(&format!(
      "inline; filename=\"{}\"",
      path.file_name().unwrap().to_str().unwrap()
    ))
    .unwrap(),
  );

  Ok((headers, tokio::fs::read(path).await.unwrap()))
}

pub(super) async fn get_thumbnail_data(
  State(ctx): State<Context>,
  Path(id): Path<Uuid>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
  let path = ctx
    .image_service
    .get_image_thumbnail_path(id)
    .await
    .unwrap()
    .ok_or(StatusCode::NOT_FOUND)?;

  let mut headers = HeaderMap::new();
  headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));

  Ok((headers, tokio::fs::read(path).await.unwrap()))
}

pub(super) async fn discover(State(ctx): State<Context>) -> Html<&'static str> {
  ctx.image_service.discover().await.unwrap();

  Html("<p>Done</p>")
}
