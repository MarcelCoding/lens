use tokio::select;
use tokio::signal::ctrl_c;
use tracing::info;

pub async fn shutdown_signal() {
  let ctrl_c = async { ctrl_c().await.expect("failed to install Ctrl+C handler") };

  #[cfg(unix)]
  {
    let terminate = async {
      tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to install signal handler")
        .recv()
        .await;
    };

    select! {
      _ = ctrl_c => {},
      _ = terminate => {},
    }
  }

  #[cfg(not(unix))]
  {
    ctrl_c.await;
  }

  info!("terminating...");
}
