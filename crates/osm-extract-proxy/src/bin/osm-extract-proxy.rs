//! The service binary (W2, W3): configure the subscriber first (OD-1), run
//! the start-up sequence (RD-1), bind `PORT`, spawn the counters row and
//! the limiter sweep, and drain for a few seconds on `SIGTERM`/`SIGINT`
//! (RD-7) before the final counters row.

use std::sync::Arc;
use std::time::{Duration, Instant};

use osm_extract_proxy::config::Config;
use osm_extract_proxy::grid::CoverageIndex;
use osm_extract_proxy::http::{AppState, ReadyData};
use osm_extract_proxy::{emit, readiness};

/// How long the drain waits for in-flight requests on shutdown (RD-7); the
/// deployment's own `RAILWAY_DEPLOYMENT_DRAINING_SECONDS` (`docs/deploy.md`)
/// should stay at or above this.
const DRAIN_SECONDS: u64 = 3;
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const COUNTERS_INTERVAL: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    // OD-1 — the subscriber is configured before anything else can log.
    emit::init_stdout();

    let config = match Config::from_env(|key| std::env::var(key).ok()) {
        Ok(config) => config,
        Err(err) => {
            // SD-9/ID-7 — invalid configuration exits non-zero, never Unready.
            eprintln!("configuration error: {err}");
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState::new(
        config.build_id.clone(),
        config.span_bound,
        Duration::from_millis(config.request_budget_ms),
        config.cache_max_bytes,
        config.requests_per_minute,
        config.requests_per_hour,
        config.cutting_slots,
    ));

    // RD-1/BR9.1 — Ready only once the manifest and every cell verify.
    match readiness::start_up(&config.manifest_path, &config.store_path) {
        Ok((manifest, store)) => {
            let coverage = CoverageIndex::from_rectangles(&manifest.coverage);
            let grid = manifest.grid_spec;
            let cell_count = manifest.cell_count;
            state.set_ready(ReadyData {
                manifest,
                store,
                coverage,
                grid,
            });
            emit::emit_ready(cell_count);
        }
        Err(reason) => {
            emit::emit_unready(reason.check, reason.failing_cells);
        }
    }

    {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(SWEEP_INTERVAL);
            loop {
                interval.tick().await;
                state.limiter_sweep(Instant::now());
            }
        });
    }
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(COUNTERS_INTERVAL);
            loop {
                interval.tick().await;
                emit::emit_counters(&state.counters().snapshot());
            }
        });
    }

    let app = osm_extract_proxy::http::router(Arc::clone(&state));
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("could not bind PORT {}: {err}", config.port);
            std::process::exit(1);
        }
    };

    let shutdown_state = Arc::clone(&state);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        wait_for_shutdown_signal().await;
        emit::emit_shutdown();
        tokio::time::sleep(Duration::from_secs(DRAIN_SECONDS)).await;
        emit::emit_counters(&shutdown_state.counters().snapshot());
    })
    .await
    .expect("the HTTP server exited unexpectedly");
}

/// `SIGTERM` (Railway's stop signal) or Ctrl-C in a local run.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing a SIGTERM handler should never fail");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
