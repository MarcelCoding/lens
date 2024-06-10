use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[clap(about, version)]
pub(super) struct Args {
  #[arg(short, long, env = "LENS_LISTEN_ADDR", default_value = "[::]:4321")]
  pub(super) listen_addr: SocketAddr,
  #[arg(short = 'u', long, env = "LENS_DB_USER", default_value = "lens")]
  pub(super) db_user: String,
  #[arg(
    short = 'p',
    long,
    env = "LENS_DB_PASS",
    conflicts_with = "db_pass_path",
    required_unless_present = "db_pass_path"
  )]
  pub(super) db_pass: Option<String>,
  #[arg(
    long,
    env = "LENS_DB_PASS_PATH",
    conflicts_with = "db_pass",
    required_unless_present = "db_pass"
  )]
  pub(super) db_pass_path: Option<String>,
  #[arg(short = 'a', long, env = "LENS_DB_ADDR", default_value = "::1")]
  pub(super) db_addr: IpAddr,
  #[arg(long, env = "LENS_DB_PORT", default_value = "5432")]
  pub(super) db_port: u16,
  #[arg(short = 'n', long, env = "LENS_DB_NAME", default_value = "lens")]
  pub(super) db_name: String,
  #[arg(short = 'm', long, env = "LENS_MEDIA_DIR")]
  pub(super) media_dir: PathBuf,
  #[arg(short = 'd', long, env = "LENS_DATA_DIR")]
  pub(super) data_dir: PathBuf,
}
