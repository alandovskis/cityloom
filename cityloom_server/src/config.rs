//! The environment, read once into a typed value.
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("{variable} is not valid: {problem}")]
    Invalid { variable: &'static str, problem: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub web_root: PathBuf,
    pub cache_control: String,
}

impl Config {
    /// `get` answers for a variable name; `std::env::var(..).ok()` in `main`, a map in tests.
    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        let host = get("E2E_HOST").unwrap_or_else(|| "127.0.0.1".into());
        let ip: IpAddr = host.parse().map_err(|_| ConfigError::Invalid { variable: "E2E_HOST", problem: "not an IP address".into() })?;
        let (variable, port) = match (get("E2E_PORT"), get("PORT")) {
            (Some(p), _) => ("E2E_PORT", p),
            (None, Some(p)) => ("PORT", p),
            (None, None) => ("E2E_PORT", "8137".into()),
        };
        let port: u16 = port.parse().map_err(|_| ConfigError::Invalid { variable, problem: "not a port number".into() })?;
        Ok(Config {
            bind: SocketAddr::new(ip, port),
            web_root: PathBuf::from(get("WEB_ROOT").unwrap_or_else(|| "web".into())),
            cache_control: get("CACHE_CONTROL").unwrap_or_else(|| "no-store".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Config::from_lookup(&|k| map.get(k).cloned())
    }

    #[test]
    fn defaults_are_loopback_8137_web_and_no_store() {
        let c = config(&[]).unwrap();
        assert_eq!(c.bind.to_string(), "127.0.0.1:8137");
        assert_eq!(c.web_root, std::path::PathBuf::from("web"));
        assert_eq!(c.cache_control, "no-store");
    }

    #[test]
    fn e2e_port_wins_over_railways_port() {
        assert_eq!(config(&[("PORT", "9000")]).unwrap().bind.port(), 9000);
        assert_eq!(config(&[("PORT", "9000"), ("E2E_PORT", "8291")]).unwrap().bind.port(), 8291);
    }

    #[test]
    fn host_cache_control_and_root_come_from_the_environment() {
        let c = config(&[("E2E_HOST", "0.0.0.0"), ("CACHE_CONTROL", "no-cache"), ("WEB_ROOT", "../web")]).unwrap();
        assert_eq!(c.bind.ip().to_string(), "0.0.0.0");
        assert_eq!((c.cache_control.as_str(), c.web_root.to_str().unwrap()), ("no-cache", "../web"));
    }

    #[test]
    fn a_port_that_is_not_a_number_names_the_variable() {
        let e = config(&[("E2E_PORT", "eighty")]).unwrap_err();
        assert_eq!(e, ConfigError::Invalid { variable: "E2E_PORT", problem: "not a port number".into() });
        assert!(e.to_string().contains("E2E_PORT"));
    }
}
