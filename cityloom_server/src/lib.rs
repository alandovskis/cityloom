pub mod accounts;
pub mod api;
pub mod app;
pub mod auth;
pub mod clock;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod db;
#[cfg(feature = "test-provider")]
pub mod fake_provider;
pub mod flow;
pub mod providers;
pub mod session;
pub mod static_files;
pub mod work;

use config::Config;

/// The account side. No connection is made here: a Postgres that is down at start-up leaves the account routes
/// answering 503 until the background task has migrated it, instead of turning the account side off for the whole
/// run. The error never holds a secret.
pub async fn build_account(config: &config::AccountConfig, clock: std::sync::Arc<dyn clock::Clock>) -> Result<std::sync::Arc<app::Account>, String> {
    let pool = db::connect_lazy(&config.database_url).map_err(|_| "DATABASE_URL could not be used".to_string())?;
    let masters = crypto::MasterKeys::new(&config.data_keys, config.data_key_current).map_err(|e| e.to_string())?;
    let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    db::spawn_migrations(pool.clone(), ready.clone());
    let repo: std::sync::Arc<dyn accounts::Repo> = std::sync::Arc::new(db::PgRepo::new(pool, masters, crypto::LookupKey::new(config.lookup_key.clone())));
    // Its first pass runs before the database may be ready and then only logs a warning; the next is a day later,
    // which is acceptable because an expired row is refused on read anyway.
    session::spawn_cleanup(repo.clone(), clock.clone());
    // `providers` is never `mut` at the top level, so a build without the feature has no `unused_mut` warning.
    #[cfg(feature = "test-provider")]
    let (providers, fake) = if config.fake_provider {
        let issuer = format!("{}/fake", config.base_url.as_str().trim_end_matches('/'));
        let mut providers = providers::Providers::from_config(config);
        for spec in fake_provider::specs(&issuer) {
            providers = providers.with(spec);
        }
        (providers, Some(fake_provider::FakeState::new(issuer, clock.clone())))
    } else {
        (providers::Providers::from_config(config), None)
    };
    #[cfg(not(feature = "test-provider"))]
    let providers = providers::Providers::from_config(config);
    Ok(std::sync::Arc::new(app::Account {
        repo,
        http: std::sync::Arc::new(providers::ReqwestClient::new()),
        clock,
        providers,
        cookie_key: cookie::Key::from(&config.cookie_key[..]),
        base_url: config.base_url.clone(),
        rate: config.rate,
        ready,
        #[cfg(feature = "test-provider")]
        fake,
    }))
}

/// With `CITYLOOM_AREAS` set: verifies and unpacks the folder's one `.cityloom` file into `DATA_ROOT/<area>` (or
/// reuses what an earlier start unpacked) and sets `config.data_mount` to that directory, for the router. Without
/// it nothing happens. An error is meant to stop the server: nothing unverified may be served.
pub async fn unpack_areas(config: &mut Config) -> std::io::Result<()> {
    let Some(areas) = config.areas.clone() else { return Ok(()) };
    let (folder, data_root) = (areas.folder.clone(), areas.data_root.clone());
    let unpacked = tokio::task::spawn_blocking(move || cityloom_prepare::archive::unpack_folder(&areas.folder, &areas.data_root))
        .await
        .map_err(std::io::Error::other)?
        .map_err(|error| {
            tracing::error!(%error, "the area archive is refused");
            std::io::Error::other(format!("CITYLOOM_AREAS {} (DATA_ROOT {}): {error}", folder.display(), data_root.display()))
        })?;
    tracing::info!(area = %unpacked.area, reused = unpacked.reused, tiles = unpacked.tiles, "area archive ready");
    config.data_mount = Some(unpacked.dir);
    Ok(())
}

/// Serves until the process is told to stop.
pub async fn run(mut config: Config) -> std::io::Result<()> {
    unpack_areas(&mut config).await?;
    let account = match &config.account {
        config::AccountSide::On(account_config) => match build_account(account_config, std::sync::Arc::new(clock::SystemClock)).await {
            Ok(account) => Some(account),
            Err(reason) => {
                tracing::warn!(%reason, "the account side is off");
                None
            }
        },
        config::AccountSide::Off(reason) => {
            tracing::warn!(%reason, "the account side is off");
            None
        }
    };
    let app = app::router(&config, account);
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %config.bind, "listening");
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).with_graceful_shutdown(shutdown_signal()).await
}

/// Ctrl-C, and on unix SIGTERM too: Railway (and Docker) stop a container with SIGTERM.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
                return;
            }
            Err(error) => tracing::warn!(%error, "SIGTERM cannot be handled; only Ctrl-C stops the server gracefully"),
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(all(test, unix))]
mod shutdown_tests {
    #[tokio::test]
    async fn sigterm_ends_the_wait() {
        let waiting = tokio::spawn(super::shutdown_signal());
        // The handler is installed when the task first runs; without it the signal would end this test process.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let status = std::process::Command::new("kill").args(["-TERM", &std::process::id().to_string()]).status().unwrap();
        assert!(status.success());
        tokio::time::timeout(std::time::Duration::from_secs(5), waiting).await.expect("SIGTERM did not end the wait").unwrap();
    }
}
