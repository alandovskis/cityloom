//! `ExtractFetcher` — fetch + classify (`components.md` `ExtractFetcher`).

use crate::failure::{FailureReason, ImportFailure};
use crate::log::log_locally;
use crate::transport::{ExtractTransport, TimeoutSource, TransportOutcome};

/// Fetches the OSM extract for an area (Contract 1) and classifies every
/// outcome — success, a typed `ApiError`, a network-level failure, or a
/// timeout — into a `Result<Vec<u8>, ImportFailure>` (BR1.1, BR2.1).
/// Generic over its transport and timeout source so tests never need a
/// live network call or a real elapsed timeout (see `transport.rs`).
pub struct ExtractFetcher<T, S> {
    transport: T,
    timeout_source: S,
    timeout_ms: u32,
}

impl<T: ExtractTransport, S: TimeoutSource> ExtractFetcher<T, S> {
    /// `timeout_ms` is a provisional value at this Unit's boundary
    /// (`security-design.md` SD-4) — the exact figure is not fixed by this
    /// Unit's design; NFR1.1's stated 10-second full-import budget is a
    /// reasonable placeholder callers may pass, but this type does not
    /// hardcode it.
    pub fn new(transport: T, timeout_source: S, timeout_ms: u32) -> ExtractFetcher<T, S> {
        ExtractFetcher {
            transport,
            timeout_source,
            timeout_ms,
        }
    }

    /// Fetches the extract for `area_id`, racing the transport's fetch
    /// future against the timeout budget (SD-4). `street_name` is the
    /// name shown at selection, carried into any `ImportFailure` (BR3.1).
    pub async fn fetch(&self, area_id: &str, street_name: &str) -> Result<Vec<u8>, ImportFailure> {
        let fetch_future = std::pin::pin!(self.transport.fetch(area_id));
        let timeout_future = std::pin::pin!(self.timeout_source.sleep(self.timeout_ms));

        match futures::future::select(fetch_future, timeout_future).await {
            futures::future::Either::Left((outcome, _)) => match outcome {
                TransportOutcome::Success(bytes) => Ok(bytes),
                TransportOutcome::ApiError(api_error) => {
                    Err(ImportFailure::new(api_error.reason, street_name))
                }
                TransportOutcome::NetworkError(detail) => {
                    log_locally(street_name, &detail);
                    Err(ImportFailure::new(
                        FailureReason::UpstreamUnavailable,
                        street_name,
                    ))
                }
            },
            futures::future::Either::Right(((), _)) => Err(ImportFailure::timeout(street_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportOutcome;
    use std::cell::Cell;
    use std::future::Future;

    /// A fake `ExtractTransport` returning a fixed, pre-recorded outcome
    /// (or, for the retry test, a different outcome on each call) — no
    /// live network call, per `unit-test-instructions.md`'s mocking
    /// guidance.
    struct FakeTransport {
        outcomes: Vec<TransportOutcome>,
        call_count: Cell<usize>,
    }

    impl FakeTransport {
        fn always(outcome: TransportOutcome) -> FakeTransport {
            FakeTransport {
                outcomes: vec![outcome],
                call_count: Cell::new(0),
            }
        }

        fn sequence(outcomes: Vec<TransportOutcome>) -> FakeTransport {
            FakeTransport {
                outcomes,
                call_count: Cell::new(0),
            }
        }
    }

    impl ExtractTransport for FakeTransport {
        fn fetch(&self, _area_id: &str) -> impl Future<Output = TransportOutcome> {
            let call = self.call_count.get();
            self.call_count.set(call + 1);
            let index = call.min(self.outcomes.len() - 1);
            let outcome = self.outcomes[index].clone();
            std::future::ready(outcome)
        }
    }

    /// A fake `TimeoutSource` that never fires — used whenever the test
    /// wants the transport to win the race deterministically.
    struct NeverTimesOut;

    impl TimeoutSource for NeverTimesOut {
        fn sleep(&self, _timeout_ms: u32) -> impl Future<Output = ()> {
            std::future::pending()
        }
    }

    /// A fake `TimeoutSource` that has already elapsed — used whenever the
    /// test wants the timeout to win the race deterministically, without
    /// a hung transport future ever needing to resolve.
    struct AlreadyTimedOut;

    impl TimeoutSource for AlreadyTimedOut {
        fn sleep(&self, _timeout_ms: u32) -> impl Future<Output = ()> {
            std::future::ready(())
        }
    }

    fn fetcher_that_never_times_out(
        transport: FakeTransport,
    ) -> ExtractFetcher<FakeTransport, NeverTimesOut> {
        ExtractFetcher::new(transport, NeverTimesOut, 10_000)
    }

    // Test 1: a successful fetch response classifies as `Ok`, not a
    // failure.
    #[test]
    fn a_successful_fetch_classifies_as_ok() {
        let fetcher =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::Success(vec![
                1, 2, 3,
            ])));

        let result = futures::executor::block_on(fetcher.fetch("area-1", "Main Street"));

        assert_eq!(result, Ok(vec![1, 2, 3]));
    }

    // Test 2: a timeout (exceeding the budget) classifies as
    // `ImportFailure { reason: timeout, retryable: true }` (BR1.1). The
    // transport future never resolves (`std::future::pending`), so this
    // is deterministic and never actually waits out a real clock.
    #[test]
    fn an_exceeded_budget_classifies_as_timeout() {
        struct NeverResponds;
        impl ExtractTransport for NeverResponds {
            fn fetch(&self, _area_id: &str) -> impl Future<Output = TransportOutcome> {
                std::future::pending()
            }
        }
        let fetcher = ExtractFetcher::new(NeverResponds, AlreadyTimedOut, 10_000);

        let result = futures::executor::block_on(fetcher.fetch("area-1", "Main Street"));

        let failure = result.expect_err("an exceeded budget must classify as a failure");
        assert_eq!(failure.reason(), FailureReason::Timeout);
        assert!(failure.retryable());
    }

    // Test 3: a rate-limited/unavailable `ApiError` classifies as
    // `rate_limited`/`upstream_unavailable`, both retryable.
    #[test]
    fn rate_limited_and_upstream_unavailable_classify_as_retryable() {
        let rate_limited =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::ApiError(
                cityloom_api_types::ApiError::without_detail(FailureReason::RateLimited),
            )));
        let unavailable =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::ApiError(
                cityloom_api_types::ApiError::without_detail(FailureReason::UpstreamUnavailable),
            )));

        let rate_limited_result =
            futures::executor::block_on(rate_limited.fetch("area-1", "Main Street"))
                .expect_err("must be a failure");
        let unavailable_result =
            futures::executor::block_on(unavailable.fetch("area-1", "Main Street"))
                .expect_err("must be a failure");

        assert_eq!(rate_limited_result.reason(), FailureReason::RateLimited);
        assert!(rate_limited_result.retryable());
        assert_eq!(
            unavailable_result.reason(),
            FailureReason::UpstreamUnavailable
        );
        assert!(unavailable_result.retryable());
    }

    // Test 4: an area-too-large/not-found `ApiError` classifies as
    // non-retryable (BR2.1).
    #[test]
    fn area_too_large_and_area_not_found_classify_as_non_retryable() {
        let too_large =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::ApiError(
                cityloom_api_types::ApiError::without_detail(FailureReason::AreaTooLarge),
            )));
        let not_found =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::ApiError(
                cityloom_api_types::ApiError::without_detail(FailureReason::AreaNotFound),
            )));

        let too_large_result =
            futures::executor::block_on(too_large.fetch("area-1", "Main Street"))
                .expect_err("must be a failure");
        let not_found_result =
            futures::executor::block_on(not_found.fetch("area-1", "Main Street"))
                .expect_err("must be a failure");

        assert!(!too_large_result.retryable());
        assert!(!not_found_result.retryable());
    }

    // Test 5: a retry after a transient failure that succeeds returns the
    // imported result, not a synthetic blank one (AC7.2.2).
    #[test]
    fn a_retry_after_a_transient_failure_returns_the_real_result() {
        let fetcher = fetcher_that_never_times_out(FakeTransport::sequence(vec![
            TransportOutcome::ApiError(cityloom_api_types::ApiError::without_detail(
                FailureReason::UpstreamUnavailable,
            )),
            TransportOutcome::Success(vec![9, 9, 9]),
        ]));

        let first = futures::executor::block_on(fetcher.fetch("area-1", "Main Street"));
        let second = futures::executor::block_on(fetcher.fetch("area-1", "Main Street"));

        assert!(first.is_err());
        assert_eq!(second, Ok(vec![9, 9, 9]));
    }

    // Test 6: `street_name` matches the name passed in at construction
    // (AC7.1.1), exercised through the fetcher's own failure path.
    #[test]
    fn a_classified_failure_carries_the_street_name_passed_to_fetch() {
        let fetcher =
            fetcher_that_never_times_out(FakeTransport::always(TransportOutcome::ApiError(
                cityloom_api_types::ApiError::without_detail(FailureReason::AreaNotFound),
            )));

        let failure =
            futures::executor::block_on(fetcher.fetch("area-1", "Northeast Pacific Street"))
                .expect_err("must be a failure");

        assert_eq!(failure.street_name(), "Northeast Pacific Street");
    }

    // Test 7: a network-level failure (no `ApiError` body available) never
    // surfaces its raw detail text to the caller — only the closed
    // `upstream_unavailable` reason (AC7.1.2, BR3.1, SD-1/SD-2).
    #[test]
    fn a_network_error_never_surfaces_raw_detail_text() {
        let fetcher = fetcher_that_never_times_out(FakeTransport::always(
            TransportOutcome::NetworkError("dns lookup failed: getaddrinfo ENOTFOUND".to_string()),
        ));

        let failure = futures::executor::block_on(fetcher.fetch("area-1", "Main Street"))
            .expect_err("must be a failure");

        assert_eq!(failure.reason(), FailureReason::UpstreamUnavailable);
        assert!(failure.retryable());
    }
}
