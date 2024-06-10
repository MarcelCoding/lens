use std::future::IntoFuture;
use std::sync::Arc;

use clap::Parser;
use sea_orm::Database;
use tokio::net::TcpListener;
use tokio::select;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use url::Url;

use migration::{Migrator, MigratorTrait};
use utils::shutdown_signal;

use crate::args::Args;
use crate::ctx::Context;
use crate::routes::router;
use crate::service::ImageService;

mod args;
mod ctx;
mod routes;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();

  let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .compact()
    .finish();

  tracing::subscriber::set_global_default(subscriber)?;

  info!(concat!(
    "booting ",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    "..."
  ));

  let mut db_url = Url::parse("postgres://a:b@c/d").unwrap();
  db_url.set_username(&args.db_user).unwrap();
  db_url
    .set_password(Some(&match args.db_pass {
      None => tokio::fs::read_to_string(args.db_pass_path.unwrap()).await?,
      Some(pass) => pass,
    }))
    .unwrap();
  db_url.set_ip_host(args.db_addr).unwrap();
  db_url.set_port(Some(args.db_port)).unwrap();
  db_url.set_path(&args.db_name);

  let db = Database::connect(db_url.as_str()).await?;

  info!("connected to db");

  Migrator::up(&db, None).await?;

  let listener = TcpListener::bind(args.listen_addr).await?;
  info!("listening at http://{}...", args.listen_addr);

  let image_service = Arc::new(ImageService::new(
    db,
    Arc::new(args.media_dir),
    Arc::new(args.data_dir.join("thumbnails")),
  ));

  let router = router()
    .with_state(Context { image_service })
    .layer(TraceLayer::new_for_http())
    .into_make_service();

  let axum = axum::serve(listener, router)
    .with_graceful_shutdown(shutdown_signal())
    .into_future();

  select! {
    result = axum => { result? }
  }

  Ok(())
}
