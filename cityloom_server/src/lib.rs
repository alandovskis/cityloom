pub mod config;
pub mod static_files;

use config::Config;

/// Serves until the process is told to stop.
pub async fn run(config: Config) -> std::io::Result<()> {
    let app = static_files::router(config.web_root.clone(), config.cache_control.clone());
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %config.bind, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
