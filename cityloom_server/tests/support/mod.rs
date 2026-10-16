#![cfg(feature = "test-provider")]
// Every file in `tests/` is its own crate and says `mod support;`, so each compiles all of this and uses only some
// of it: `InProcess` is unused in `static_files.rs`, `Browser` in `providers.rs`. CI builds with
// `RUSTFLAGS=-D warnings`, which would turn those `dead_code` warnings into errors; this is the one place that
// allows them, and it covers everything added to the file later.
#![allow(dead_code)]
//! Plays the network and the browser for the integration tests.
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use cityloom_server::clock::FixedClock;
use cityloom_server::fake_provider::{self, FakeState};
use cityloom_server::providers::HttpClient;
use http_body_util::BodyExt;
use tower::ServiceExt;

pub const ISSUER: &str = "http://127.0.0.1:8137/fake";

pub fn t0() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
}

/// The fake provider, answering as if it were on the network.
pub struct InProcess(pub Router);

impl InProcess {
    pub fn new(clock: Arc<FixedClock>) -> InProcess {
        InProcess(Router::new().nest("/fake", fake_provider::router(FakeState::new(ISSUER.into(), clock))))
    }
}

#[async_trait::async_trait]
impl HttpClient for InProcess {
    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String> {
        let (parts, body) = request.into_parts();
        let res = self.0.clone().oneshot(http::Request::from_parts(parts, Body::from(body))).await.map_err(|e| e.to_string())?;
        let (parts, body) = res.into_parts();
        Ok(http::Response::from_parts(parts, body.collect().await.map_err(|e| e.to_string())?.to_bytes().to_vec()))
    }
}

/// A client for a provider that is down.
pub struct Down;

#[async_trait::async_trait]
impl HttpClient for Down {
    async fn send(&self, _: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String> {
        Err("connection refused".into())
    }
}
