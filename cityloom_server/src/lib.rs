pub mod accounts;
pub mod app;
pub mod config;
pub mod crypto;
pub mod static_files;

use config::Config;

/// Serves until the process is told to stop.
pub async fn run(config: Config) -> std::io::Result<()> {
    if let config::AccountSide::Off(reason) = &config.account {
        tracing::warn!(%reason, "the account side is off");
    }
    let app = app::router(&config);
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %config.bind, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
