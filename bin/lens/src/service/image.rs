use std::fs::Metadata;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::anyhow;
use sea_orm::{
  ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
  QueryFilter, QueryOrder, QuerySelect,
};
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::error;
use uuid::Uuid;

use entity::prelude::Image;

enum TaskResult<T> {
  New(T),
  Update(entity::image::Model, T),
}

struct IndexResult {
  width: u32,
  height: u32,
  file_size: u64,
  taken: Option<OffsetDateTime>,
  modified: Option<OffsetDateTime>,
  path: String,
}

impl<T, E> TaskResult<Result<T, E>> {
  fn transpose(self) -> Result<TaskResult<T>, E> {
    Ok(match self {
      TaskResult::New(value) => TaskResult::New(value?),
      TaskResult::Update(image, value) => TaskResult::Update(image, value?),
    })
  }
}

pub(crate) struct ImageService {
  db: DatabaseConnection,
  media_dir: Arc<PathBuf>,
  thumbnail_dir: Arc<PathBuf>,
}

impl ImageService {
  pub(crate) fn new(
    db: DatabaseConnection,
    media_dir: Arc<PathBuf>,
    thumbnail_dir: Arc<PathBuf>,
  ) -> Self {
    Self {
      db,
      media_dir,
      thumbnail_dir,
    }
  }

  pub(crate) async fn list(
    &self,
    from: OffsetDateTime,
    to: Option<OffsetDateTime>,
    limit: u64,
  ) -> anyhow::Result<Vec<entity::image::Model>> {
    let to = to.unwrap_or_else(|| OffsetDateTime::now_utc());

    // TODO: handle images without taken...
    Ok(
      Image::find()
        .filter(
          entity::image::Column::Taken
            .is_not_null()
            .and(
              entity::image::Column::Taken
                .gte(from)
                .and(entity::image::Column::Taken.lte(to)),
            )
            .or(
              entity::image::Column::Taken.is_null().and(
                entity::image::Column::Modified
                  .gte(from)
                  .and(entity::image::Column::Modified.lte(to)),
              ),
            ),
        )
        .order_by_asc(entity::image::Column::Taken)
        .limit(limit)
        .all(&self.db)
        .await?,
    )
  }

  pub(crate) async fn get_image_by_path(
    &self,
    path: &Path,
  ) -> anyhow::Result<Option<entity::image::Model>> {
    let path = path.to_str().ok_or_else(|| anyhow!("non-utf8 path"))?;

    Ok(
      Image::find()
        .filter(entity::image::Column::Path.eq(path))
        .one(&self.db)
        .await?,
    )
  }

  pub(crate) async fn get_image_path(&self, id: Uuid) -> anyhow::Result<Option<PathBuf>> {
    match Image::find_by_id(id).one(&self.db).await? {
      None => Ok(None),
      Some(image) => Ok(Some(self.media_dir.join(image.path))),
    }
  }

  pub(crate) async fn get_image_thumbnail_path(&self, id: Uuid) -> anyhow::Result<Option<PathBuf>> {
    match Image::find_by_id(id).one(&self.db).await? {
      None => Ok(None),
      Some(image) => Ok(Some(
        self
          .thumbnail_dir
          .join(image.id.to_string())
          .with_extension("webp"),
      )),
    }
  }

  pub(crate) async fn discover(&self) -> anyhow::Result<()> {
    // TODO: handle deleted files

    let mut folders = vec![self.media_dir.clone()];

    let mut tasks = JoinSet::new();

    while let Some(folder) = folders.pop() {
      let mut dir = tokio::fs::read_dir(folder.as_ref()).await?;

      while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let path = entry.path();

        if metadata.is_dir() {
          folders.push(Arc::new(path));
        } else if is_image(&path) {
          let relative_path = path.strip_prefix(self.media_dir.as_ref())?.to_path_buf();
          let image = self.get_image_by_path(&relative_path).await?;

          match image {
            None => {
              tasks.spawn(async move {
                TaskResult::New(index_image(&path, &relative_path, metadata).await)
              });
            }
            Some(image) => {
              let size = metadata.len();

              if image.file_size as u64 != size {
                tasks.spawn(async move {
                  TaskResult::Update(image, index_image(&path, &relative_path, metadata).await)
                });
              } else {
                let modified = if let Some(modified) = image.modified {
                  match metadata.modified() {
                    Ok(last_modified) => OffsetDateTime::from(last_modified) != modified,
                    Err(err) => {
                      error!(
                        "Unable to get last modified of {}: {}",
                        relative_path.to_string_lossy(),
                        err
                      );
                      false
                    }
                  }
                } else {
                  false
                };

                if modified {
                  tasks.spawn(async move {
                    let path = path;
                    TaskResult::Update(image, index_image(&path, &relative_path, metadata).await)
                  });
                }
              }
            }
          }
        }
      }

      while let Some(image) = tasks.join_next().await.transpose()? {
        match image.transpose() {
          Ok(TaskResult::New(result)) => {
            let active = entity::image::ActiveModel {
              id: ActiveValue::NotSet,
              created: ActiveValue::NotSet,
              updated: ActiveValue::NotSet,
              path: ActiveValue::Set(result.path),
              // TODO: error on overflow
              width: ActiveValue::Set(result.width as i32),
              height: ActiveValue::Set(result.height as i32),
              file_size: ActiveValue::Set(result.file_size as i64),
              thumbnail: ActiveValue::NotSet,
              taken: ActiveValue::Set(result.taken),
              modified: ActiveValue::Set(result.modified),
            };

            active.insert(&self.db).await?;
          }
          Ok(TaskResult::Update(image, result)) => {
            let mut image = image.into_active_model();
            // TODO: catch overflow
            image.file_size = ActiveValue::Set(result.file_size as i64);
            image.width = ActiveValue::Set(result.width as i32);
            image.height = ActiveValue::Set(result.height as i32);
            image.taken = ActiveValue::Set(result.taken);
            image.modified = ActiveValue::Set(result.modified);
            image.path = ActiveValue::Set(result.path);

            image.update(&self.db).await?;
          }
          Err(err) => error!("Error indexing image: {}", err),
        }
      }
    }

    Ok(())
  }
}

fn is_image(path: &Path) -> bool {
  matches!(
    path.extension().and_then(|extension| extension.to_str()),
    Some("gif" | "jpg" | "png" | "webp")
  )
}

async fn index_image(
  path: &Path,
  relative_path: &Path,
  metadata: Metadata,
) -> anyhow::Result<IndexResult> {
  let image = {
    let data = tokio::fs::read(&path).await?;
    let cursor = Cursor::new(data.clone());
    image::io::Reader::new(cursor)
      .with_guessed_format()?
      .decode()?
  };

  let width = image.width();
  let height = image.height();

  //let mut cursor = Cursor::new(data);
  //let exif = exif::Reader::new().read_from_container(&mut cursor)?;
  //if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
  //    if let Value::Ascii(value) = &field.value {
  //let next =
  //println!("{}", std::str::from_utf8(&v[..])?);
  //                  }
  //            }
  //      }

  let relative_path = relative_path
    .to_str()
    .ok_or_else(|| anyhow!("path not valid utf8"))?
    .to_string();

  Ok::<_, anyhow::Error>(IndexResult {
    width,
    height,
    file_size: metadata.len(),
    taken: Option::None,
    modified: match metadata.modified() {
      Ok(time) => Some(time.into()),
      Err(err) => {
        error!("Unable to get last modified of {}: {}", relative_path, err);
        None
      }
    },
    path: relative_path,
  })
}
/*
fn generate_thumbnail() {
  let mut image = {
    let data = tokio::fs::read(&path).await?;
    let cursor = Cursor::new(data.clone());
    image::io::Reader::new(cursor)
      .with_guessed_format()?
      .decode()?
  };

  let width = image.width();
  let height = image.height();

  // generate thumbnail
  if width < height {
    image = image.crop(0, (height - width) / 2, width, width);
  } else if width > height {
    image = image.crop((width - height) / 2, 0, height, height);
  }

  let id = Uuid::new_v4();
  {
    let thumbnail = image.resize_exact(300, 300, FilterType::Lanczos3);
    let mut thumbnail_data = Vec::new();
    thumbnail.write_to(
      &mut Cursor::new(&mut thumbnail_data),
      image::ImageFormat::WebP,
    )?;

    let thumbnail_path = thumbnail_dir.join(id.to_string()).with_extension("webp");
    if let Some(path) = thumbnail_path.parent() {
      if !tokio::fs::try_exists(path).await? {
        tokio::fs::create_dir_all(path).await?;
      }
    }

    tokio::fs::write(thumbnail_path, thumbnail_data).await?;
  }
}
*/
