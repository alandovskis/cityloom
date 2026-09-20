//! `config` — typed `CITYLOOM_*` environment configuration, fail-fast
//! (SD-9, ID-7). Invalid configuration is a start-up error, never a
//! degraded Unready state (BR9.1 is about the data build, not
//! configuration).

use std::path::PathBuf;

use thiserror::Error;

/// A configuration value was missing or did not parse (fail-fast: the
/// process exits non-zero rather than starting Unready).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("configuration value {0} is missing or invalid")]
pub struct ConfigError(pub &'static str);

/// The service's start-up configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    pub store_path: PathBuf,
    pub manifest_path: PathBuf,
    /// The build stamp the `x-client-build` header is checked against
    /// (BR1.1): `RAILWAY_GIT_COMMIT_SHA` if present, else
    /// `CITYLOOM_BUILD_ID`, validated against `[A-Za-z0-9._-]{1,64}`.
    pub build_id: String,
    pub cache_max_bytes: u64,
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub span_bound: u64,
    pub cutting_slots: usize,
    pub request_budget_ms: u64,
}

const DEFAULT_CACHE_MAX_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 30;
const DEFAULT_REQUESTS_PER_HOUR: u32 = 300;
const DEFAULT_SPAN_BOUND: u64 = 12;
const DEFAULT_CUTTING_SLOTS: usize = 4;
const DEFAULT_REQUEST_BUDGET_MS: u64 = 3_000;
const DEFAULT_PORT: u16 = 8080;

impl Config {
    /// Read configuration through `get` (`std::env::var` in production; a
    /// `HashMap` lookup in tests), failing fast on the first invalid value.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        let port = match get("PORT") {
            Some(text) => text.parse().map_err(|_| ConfigError("PORT"))?,
            None => DEFAULT_PORT,
        };
        let store_path = get("CITYLOOM_STORE_PATH")
            .ok_or(ConfigError("CITYLOOM_STORE_PATH"))?
            .into();
        let manifest_path = get("CITYLOOM_MANIFEST_PATH")
            .ok_or(ConfigError("CITYLOOM_MANIFEST_PATH"))?
            .into();
        let build_id = get("RAILWAY_GIT_COMMIT_SHA")
            .or_else(|| get("CITYLOOM_BUILD_ID"))
            .ok_or(ConfigError("RAILWAY_GIT_COMMIT_SHA or CITYLOOM_BUILD_ID"))?;
        if !is_valid_build_stamp(&build_id) {
            return Err(ConfigError("RAILWAY_GIT_COMMIT_SHA or CITYLOOM_BUILD_ID"));
        }
        let cache_max_bytes =
            parse_or_default(&get, "CITYLOOM_CACHE_MAX_BYTES", DEFAULT_CACHE_MAX_BYTES)?;
        let requests_per_minute = parse_or_default(
            &get,
            "CITYLOOM_REQUESTS_PER_MINUTE",
            DEFAULT_REQUESTS_PER_MINUTE,
        )?;
        let requests_per_hour = parse_or_default(
            &get,
            "CITYLOOM_REQUESTS_PER_HOUR",
            DEFAULT_REQUESTS_PER_HOUR,
        )?;
        let span_bound = parse_or_default(&get, "CITYLOOM_SPAN_BOUND", DEFAULT_SPAN_BOUND)?;
        let cutting_slots =
            parse_or_default(&get, "CITYLOOM_CUTTING_SLOTS", DEFAULT_CUTTING_SLOTS)?;
        let request_budget_ms = parse_or_default(
            &get,
            "CITYLOOM_REQUEST_BUDGET_MS",
            DEFAULT_REQUEST_BUDGET_MS,
        )?;

        Ok(Config {
            port,
            store_path,
            manifest_path,
            build_id,
            cache_max_bytes,
            requests_per_minute,
            requests_per_hour,
            span_bound,
            cutting_slots,
            request_budget_ms,
        })
    }
}

fn parse_or_default<T: std::str::FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    key: &'static str,
    default: T,
) -> Result<T, ConfigError> {
    match get(key) {
        Some(text) => text.parse().map_err(|_| ConfigError(key)),
        None => Ok(default),
    }
}

/// `[A-Za-z0-9._-]{1,64}` (BR1.1's build-stamp shape).
fn is_valid_build_stamp(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn a_complete_environment_parses_with_every_value_taken_literally() {
        let config = Config::from_env(env(&[
            ("PORT", "9090"),
            ("CITYLOOM_STORE_PATH", "/data/store.bin"),
            ("CITYLOOM_MANIFEST_PATH", "/data/manifest.json"),
            ("CITYLOOM_BUILD_ID", "abc123"),
            ("CITYLOOM_CACHE_MAX_BYTES", "1000"),
            ("CITYLOOM_REQUESTS_PER_MINUTE", "5"),
        ]))
        .unwrap();
        assert_eq!(config.port, 9090);
        assert_eq!(config.store_path, PathBuf::from("/data/store.bin"));
        assert_eq!(config.build_id, "abc123");
        assert_eq!(config.cache_max_bytes, 1_000);
        assert_eq!(config.requests_per_minute, 5);
        // Untouched values keep their stated defaults.
        assert_eq!(config.requests_per_hour, DEFAULT_REQUESTS_PER_HOUR);
        assert_eq!(config.span_bound, DEFAULT_SPAN_BOUND);
        assert_eq!(config.cutting_slots, DEFAULT_CUTTING_SLOTS);
        assert_eq!(config.request_budget_ms, DEFAULT_REQUEST_BUDGET_MS);
    }

    #[test]
    fn a_missing_required_value_fails_fast() {
        let err = Config::from_env(env(&[
            ("CITYLOOM_MANIFEST_PATH", "/x"),
            ("CITYLOOM_BUILD_ID", "a"),
        ]))
        .unwrap_err();
        assert_eq!(err.0, "CITYLOOM_STORE_PATH");
    }

    #[test]
    fn a_missing_build_stamp_fails_fast() {
        let err = Config::from_env(env(&[
            ("CITYLOOM_STORE_PATH", "/x"),
            ("CITYLOOM_MANIFEST_PATH", "/y"),
        ]))
        .unwrap_err();
        assert_eq!(err.0, "RAILWAY_GIT_COMMIT_SHA or CITYLOOM_BUILD_ID");
    }

    // BR1.1 — the build stamp must be [A-Za-z0-9._-]{1,64}.
    #[test]
    fn an_invalid_build_stamp_fails_fast() {
        for bad in ["", "has space", "sl/ash", &"a".repeat(65)] {
            let err = Config::from_env(env(&[
                ("CITYLOOM_STORE_PATH", "/x"),
                ("CITYLOOM_MANIFEST_PATH", "/y"),
                ("CITYLOOM_BUILD_ID", bad),
            ]))
            .unwrap_err();
            assert_eq!(
                err.0, "RAILWAY_GIT_COMMIT_SHA or CITYLOOM_BUILD_ID",
                "{bad:?}"
            );
        }
    }

    // RAILWAY_GIT_COMMIT_SHA is preferred over CITYLOOM_BUILD_ID.
    #[test]
    fn railway_git_commit_sha_is_preferred_over_cityloom_build_id() {
        let config = Config::from_env(env(&[
            ("CITYLOOM_STORE_PATH", "/x"),
            ("CITYLOOM_MANIFEST_PATH", "/y"),
            ("RAILWAY_GIT_COMMIT_SHA", "from-railway"),
            ("CITYLOOM_BUILD_ID", "from-cityloom"),
        ]))
        .unwrap();
        assert_eq!(config.build_id, "from-railway");
    }

    #[test]
    fn an_unparseable_numeric_value_fails_fast() {
        let err = Config::from_env(env(&[
            ("CITYLOOM_STORE_PATH", "/x"),
            ("CITYLOOM_MANIFEST_PATH", "/y"),
            ("CITYLOOM_BUILD_ID", "a"),
            ("PORT", "not-a-port"),
        ]))
        .unwrap_err();
        assert_eq!(err.0, "PORT");
    }
}
