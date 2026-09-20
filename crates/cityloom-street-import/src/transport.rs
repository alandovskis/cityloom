//! `ExtractTransport` and `TimeoutSource` — the two ports `ExtractFetcher`
//! is generic over (`security-design.md` SD-4), plus their `gloo`-backed
//! implementations for the real browser build.
//!
//! Unit tests exercise `ExtractFetcher` through fakes of these traits and
//! never construct [`GlooExtractTransport`]/[`GlooTimeoutSource`]: awaiting
//! a real `gloo-net`/`gloo-timers` future outside a browser's JS engine
//! aborts the process (verified empirically against this crate's own
//! native `cargo test` target during this Unit's build — `web-sys`'s
//! generated bindings have nothing to call into on a target other than
//! `wasm32-unknown-unknown` running inside a JS host). Keeping the browser
//! transport and the timeout race behind these two small ports is what
//! makes `ExtractFetcher`'s own logic (classification, the timeout race
//! outcome) testable natively at all.

use std::future::Future;

use cityloom_api_types::ApiError;

/// What a fetch attempt produced, before `ExtractFetcher` classifies it
/// into an [`crate::ImportFailure`]. Never a bare HTTP status code: Contract
/// 1's non-2xx responses carry a typed `ApiError` body, so a real
/// transport failure already carries a closed `FailureReason` by the time
/// it reaches this type.
#[derive(Clone, Debug, PartialEq)]
pub enum TransportOutcome {
    /// The raw extract bytes (BR7.1 — the only network operation this
    /// crate performs is this read).
    Success(Vec<u8>),
    /// A non-2xx response whose body parsed as Contract 1's `ApiError`.
    ApiError(ApiError),
    /// The request could not complete at all (offline, DNS failure, a
    /// response whose body did not parse as `ApiError`) — no typed reason
    /// is available from the transport itself; `ExtractFetcher` classifies
    /// this as `upstream_unavailable` (retryable) and logs `detail`
    /// locally (SD-1/SD-2), never returning it to the caller.
    NetworkError(String),
}

/// The fetch port `ExtractFetcher` is generic over (BR7.1: every
/// implementation only ever issues a `GET`).
pub trait ExtractTransport {
    /// Issues the Contract 1 extract-fetch call for `area_id`.
    fn fetch(&self, area_id: &str) -> impl Future<Output = TransportOutcome>;
}

/// The timeout-budget port `ExtractFetcher` races the transport's fetch
/// future against (`security-design.md` SD-4).
pub trait TimeoutSource {
    /// Resolves after `timeout_ms` milliseconds.
    fn sleep(&self, timeout_ms: u32) -> impl Future<Output = ()>;
}

/// The real, `gloo-net`-backed `ExtractTransport` for the browser build.
/// `base_url` is the Contract 1 extract-fetch endpoint's own base (e.g.
/// `https://api.example.org`); this type issues exactly one `GET
/// {base_url}/api/extract?area={area_id}` per `fetch` call and nothing
/// else (BR7.1 — never a write).
#[derive(Clone, Debug)]
pub struct GlooExtractTransport {
    base_url: String,
}

impl GlooExtractTransport {
    pub fn new(base_url: impl Into<String>) -> GlooExtractTransport {
        GlooExtractTransport {
            base_url: base_url.into(),
        }
    }
}

impl ExtractTransport for GlooExtractTransport {
    async fn fetch(&self, area_id: &str) -> TransportOutcome {
        let url = format!("{}/api/extract?area={area_id}", self.base_url);
        let response = match gloo_net::http::Request::get(&url).send().await {
            Ok(response) => response,
            Err(error) => return TransportOutcome::NetworkError(error.to_string()),
        };

        if response.ok() {
            return match response.binary().await {
                Ok(bytes) => TransportOutcome::Success(bytes),
                Err(error) => TransportOutcome::NetworkError(error.to_string()),
            };
        }

        match response.json::<ApiError>().await {
            Ok(api_error) => TransportOutcome::ApiError(api_error),
            Err(error) => TransportOutcome::NetworkError(error.to_string()),
        }
    }
}

/// The real, `gloo-timers`-backed `TimeoutSource` for the browser build
/// (`security-design.md` SD-4).
#[derive(Clone, Copy, Debug, Default)]
pub struct GlooTimeoutSource;

impl TimeoutSource for GlooTimeoutSource {
    async fn sleep(&self, timeout_ms: u32) {
        gloo_timers::future::TimeoutFuture::new(timeout_ms).await;
    }
}
