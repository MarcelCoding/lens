use crate::service::ImageService;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct Context {
  pub(crate) image_service: Arc<ImageService>,
}
