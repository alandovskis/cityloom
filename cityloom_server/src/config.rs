//! The environment, read once into a typed value.
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use zeroize::Zeroizing;

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
    pub account: AccountSide,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum OffReason {
    #[error("DATABASE_URL is not set")]
    NoDatabase,
    #[error("{0} is not set")]
    Missing(&'static str),
    #[error("{variable} is not valid: {problem}")]
    Invalid { variable: &'static str, problem: String },
}

/// Whether the accounts work. A bad account variable turns them off; the site still starts.
#[derive(Debug, Clone)]
pub enum AccountSide {
    On(Box<AccountConfig>),
    Off(OffReason),
}

/// The three key fields are wiped when the config drops, and so are the copies of it.
#[derive(Clone)]
pub struct AccountConfig {
    pub base_url: url::Url,
    pub database_url: String,
    pub cookie_key: Zeroizing<[u8; 64]>,
    pub data_keys: Vec<(u8, Zeroizing<[u8; 32]>)>,
    pub data_key_current: u8,
    pub lookup_key: Zeroizing<[u8; 32]>,
    pub providers: Vec<ProviderCreds>,
    pub rate: RateLimit,
    pub fake_provider: bool,
}

#[derive(Clone)]
pub struct ProviderCreds {
    pub slug: &'static str,
    pub client_id: String,
    pub secret: ProviderSecret,
}

#[derive(Clone)]
pub enum ProviderSecret {
    Static(String),
    Apple { team_id: String, key_id: String, private_key_pem: String },
}

impl std::fmt::Debug for ProviderCreds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCreds").field("slug", &self.slug).finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ProviderSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProviderSecret::Static(_) => "ProviderSecret::Static",
            ProviderSecret::Apple { .. } => "ProviderSecret::Apple",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateLimit {
    pub period_secs: u64,
    pub burst: u32,
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
            account: account_side(get),
        })
    }
}

const DEFAULT_PERIOD_SECS: u64 = 6; // 10 a minute
const DEFAULT_BURST: u32 = 20;

/// Key bytes come out in memory that is wiped when it drops (`Zeroizing`), whole or split.
fn decode<const N: usize>(variable: &'static str, text: &str) -> Result<Zeroizing<[u8; N]>, OffReason> {
    use base64::Engine;
    let bytes = Zeroizing::new(
        base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(|_| OffReason::Invalid { variable, problem: "not base64".into() })?,
    );
    if bytes.len() != N {
        return Err(OffReason::Invalid { variable, problem: format!("{} bytes were given, {N} are needed", bytes.len()) });
    }
    let mut key = Zeroizing::new([0u8; N]);
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn required(get: &dyn Fn(&str) -> Option<String>, variable: &'static str) -> Result<String, OffReason> {
    get(variable).filter(|v| !v.trim().is_empty()).ok_or(OffReason::Missing(variable))
}

fn account_side(get: &dyn Fn(&str) -> Option<String>) -> AccountSide {
    match account(get) {
        Ok(a) => AccountSide::On(Box::new(a)),
        Err(reason) => AccountSide::Off(reason),
    }
}

fn account(get: &dyn Fn(&str) -> Option<String>) -> Result<AccountConfig, OffReason> {
    let database_url = get("DATABASE_URL").filter(|v| !v.trim().is_empty()).ok_or(OffReason::NoDatabase)?;
    let base_url = required(get, "BASE_URL")?;
    let base_url = url::Url::parse(&base_url).map_err(|_| OffReason::Invalid { variable: "BASE_URL", problem: "not a URL".into() })?;
    let cookie_key = decode::<64>("COOKIE_KEY", &required(get, "COOKIE_KEY")?)?;
    let mut data_keys: Vec<(u8, Zeroizing<[u8; 32]>)> = Vec::new();
    for entry in required(get, "DATA_KEYS")?.split(',') {
        // Nothing of the entry is put in the error: it may be a key.
        let bad = |problem: &str| OffReason::Invalid { variable: "DATA_KEYS", problem: problem.into() };
        let (id, key) = entry.trim().split_once(':').ok_or_else(|| bad("each key is <id>:<base64>"))?;
        let id: u8 = id.parse().map_err(|_| bad("a key id is a number from 0 to 255"))?;
        if data_keys.iter().any(|(i, _)| *i == id) {
            return Err(bad("a key id is used twice"));
        }
        data_keys.push((id, decode::<32>("DATA_KEYS", key)?));
    }
    let current = required(get, "DATA_KEY_CURRENT")?;
    let data_key_current: u8 = current.trim().parse().map_err(|_| OffReason::Invalid { variable: "DATA_KEY_CURRENT", problem: "not a key id".into() })?;
    if !data_keys.iter().any(|(i, _)| *i == data_key_current) {
        return Err(OffReason::Invalid { variable: "DATA_KEY_CURRENT", problem: "no key in DATA_KEYS has this id".into() });
    }
    let lookup_key = decode::<32>("LOOKUP_KEY", &required(get, "LOOKUP_KEY")?)?;
    // A period or a burst of 0, or a burst too big for a u32, would make the rate limiter refuse to build, so they
    // turn the accounts off here instead of stopping the whole server there.
    let positive = |variable: &'static str, default: u32| -> Result<u32, OffReason> {
        let Some(v) = get(variable).filter(|v| !v.trim().is_empty()) else { return Ok(default) };
        match v.trim().parse::<u32>() {
            Ok(n) if n > 0 => Ok(n),
            _ => Err(OffReason::Invalid { variable, problem: "a whole number from 1 to 4294967295".into() }),
        }
    };
    let rate = RateLimit { period_secs: positive("AUTH_RATE_PERIOD_SECS", DEFAULT_PERIOD_SECS as u32)? as u64, burst: positive("AUTH_RATE_BURST", DEFAULT_BURST)? };
    let pair = |slug: &'static str, id: &str, secret: &str| -> Option<ProviderCreds> {
        Some(ProviderCreds { slug, client_id: get(id)?, secret: ProviderSecret::Static(get(secret)?) })
    };
    let mut providers: Vec<ProviderCreds> = [
        ("osm", "OSM_CLIENT_ID", "OSM_CLIENT_SECRET"),
        ("google", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"),
        ("microsoft", "MICROSOFT_CLIENT_ID", "MICROSOFT_CLIENT_SECRET"),
    ]
    .into_iter()
    .filter_map(|(slug, id, secret)| pair(slug, id, secret))
    .collect();
    if let (Some(client_id), Some(team_id), Some(key_id), Some(private_key_pem)) =
        (get("APPLE_CLIENT_ID"), get("APPLE_TEAM_ID"), get("APPLE_KEY_ID"), get("APPLE_PRIVATE_KEY"))
    {
        // A host that wants one line for a variable has the PEM's line breaks written as \n.
        providers.push(ProviderCreds {
            slug: "apple",
            client_id,
            secret: ProviderSecret::Apple { team_id, key_id, private_key_pem: private_key_pem.replace("\\n", "\n") },
        });
    }
    Ok(AccountConfig {
        base_url,
        database_url,
        cookie_key,
        data_keys,
        data_key_current,
        lookup_key,
        providers,
        rate,
        fake_provider: get("FAKE_PROVIDER").as_deref() == Some("1"),
    })
}

impl std::fmt::Debug for AccountConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountConfig")
            .field("base_url", &self.base_url.as_str())
            .field("providers", &self.providers.iter().map(|p| p.slug).collect::<Vec<_>>())
            .finish_non_exhaustive()
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

    use base64::Engine;

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn account_vars() -> Vec<(String, String)> {
        vec![
            ("DATABASE_URL".into(), "postgres://u:p@localhost/db".into()),
            ("BASE_URL".into(), "http://127.0.0.1:8137".into()),
            ("COOKIE_KEY".into(), b64(&[7u8; 64])),
            ("DATA_KEYS".into(), format!("1:{},2:{}", b64(&[1u8; 32]), b64(&[2u8; 32]))),
            ("DATA_KEY_CURRENT".into(), "2".into()),
            ("LOOKUP_KEY".into(), b64(&[9u8; 32])),
        ]
    }

    fn with(vars: &[(String, String)], without: &[&str], extra: &[(&str, &str)]) -> Config {
        let mut m: HashMap<String, String> = vars.iter().cloned().collect();
        for k in without {
            m.remove(*k);
        }
        for (k, v) in extra {
            m.insert(k.to_string(), v.to_string());
        }
        Config::from_lookup(&|k| m.get(k).cloned()).unwrap()
    }

    fn off(c: &Config) -> &OffReason {
        match &c.account {
            AccountSide::Off(r) => r,
            AccountSide::On(_) => panic!("the account side should be off"),
        }
    }

    #[test]
    fn without_a_database_the_account_side_is_off() {
        assert_eq!(off(&with(&account_vars(), &["DATABASE_URL"], &[])), &OffReason::NoDatabase);
        assert_eq!(off(&config_plain()), &OffReason::NoDatabase);
    }

    fn config_plain() -> Config {
        config(&[]).unwrap()
    }

    #[test]
    fn each_missing_key_variable_is_named() {
        for (var, reason) in [
            ("BASE_URL", OffReason::Missing("BASE_URL")),
            ("COOKIE_KEY", OffReason::Missing("COOKIE_KEY")),
            ("DATA_KEYS", OffReason::Missing("DATA_KEYS")),
            ("DATA_KEY_CURRENT", OffReason::Missing("DATA_KEY_CURRENT")),
            ("LOOKUP_KEY", OffReason::Missing("LOOKUP_KEY")),
        ] {
            let c = with(&account_vars(), &[var], &[]);
            assert_eq!(off(&c), &reason);
            assert!(off(&c).to_string().contains(var));
        }
    }

    #[test]
    fn a_key_of_the_wrong_length_or_a_current_id_that_is_not_listed_is_invalid_and_never_echoed() {
        let c = with(&account_vars(), &[], &[("LOOKUP_KEY", &b64(&[1u8; 31]))]);
        assert!(matches!(off(&c), OffReason::Invalid { variable: "LOOKUP_KEY", .. }));
        let c = with(&account_vars(), &[], &[("DATA_KEY_CURRENT", "3")]);
        assert!(matches!(off(&c), OffReason::Invalid { variable: "DATA_KEY_CURRENT", .. }));
        let c = with(&account_vars(), &[], &[("COOKIE_KEY", &b64(&[7u8; 63]))]);
        assert!(matches!(off(&c), OffReason::Invalid { variable: "COOKIE_KEY", .. }));
        let c = with(&account_vars(), &[], &[("DATA_KEYS", "1:AAAA,1:AAAA")]);
        assert!(!off(&c).to_string().contains("AAAA"));
    }

    #[test]
    fn with_everything_set_the_account_side_is_on_and_providers_need_all_their_variables() {
        let c = with(
            &account_vars(),
            &[],
            &[
                ("GOOGLE_CLIENT_ID", "gid"),
                ("GOOGLE_CLIENT_SECRET", "gs"),
                ("OSM_CLIENT_ID", "only-id"),
                ("APPLE_CLIENT_ID", "a"),
                ("APPLE_TEAM_ID", "t"),
                ("APPLE_KEY_ID", "k"),
                ("APPLE_PRIVATE_KEY", "pem"),
            ],
        );
        let AccountSide::On(a) = &c.account else { panic!("{:?}", c.account) };
        let slugs: Vec<_> = a.providers.iter().map(|p| p.slug).collect();
        assert_eq!(slugs, ["google", "apple"]);
        assert_eq!(a.data_key_current, 2);
        assert_eq!(a.data_keys.len(), 2);
        assert_eq!((a.rate.period_secs, a.rate.burst), (6, 20));
        assert_eq!(a.base_url.as_str(), "http://127.0.0.1:8137/");
        assert!(!format!("{a:?}").contains("gs"), "Debug must not print secrets");
    }

    #[test]
    fn the_apple_private_key_may_have_its_line_breaks_written_as_backslash_n() {
        let c = with(
            &account_vars(),
            &[],
            &[
                ("APPLE_CLIENT_ID", "a"),
                ("APPLE_TEAM_ID", "t"),
                ("APPLE_KEY_ID", "k"),
                ("APPLE_PRIVATE_KEY", "-----BEGIN PRIVATE KEY-----\\nAAAA\\n-----END PRIVATE KEY-----"),
            ],
        );
        let AccountSide::On(a) = &c.account else { panic!() };
        let ProviderSecret::Apple { private_key_pem, .. } = &a.providers[0].secret else { panic!() };
        assert_eq!(private_key_pem, "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----");
    }

    #[test]
    fn key_material_is_held_in_types_that_wipe_it_when_they_drop_and_so_are_its_copies() {
        // A compile-time check: change a field to a plain array and this stops compiling.
        fn wipes_on_drop<T: zeroize::ZeroizeOnDrop>(_: &T) {}
        let c = with(&account_vars(), &[], &[]);
        let AccountSide::On(a) = &c.account else { panic!() };
        wipes_on_drop(&a.cookie_key);
        wipes_on_drop(&a.lookup_key);
        for (_, key) in &a.data_keys {
            wipes_on_drop(key);
        }
        // `Config: Clone` copies the keys into `Zeroizing`s again.
        let copy = c.clone();
        let AccountSide::On(b) = &copy.account else { panic!() };
        wipes_on_drop(&b.cookie_key);
        assert_eq!(&b.cookie_key[..], &a.cookie_key[..]);
    }

    #[test]
    fn a_rate_limit_that_cannot_be_built_turns_the_accounts_off_naming_the_variable() {
        for (variable, value) in [
            ("AUTH_RATE_BURST", "0"),
            ("AUTH_RATE_BURST", "4294967296"),
            ("AUTH_RATE_BURST", "-1"),
            ("AUTH_RATE_BURST", "many"),
            ("AUTH_RATE_PERIOD_SECS", "0"),
            ("AUTH_RATE_PERIOD_SECS", "4294967296"),
        ] {
            let c = with(&account_vars(), &[], &[(variable, value)]);
            assert!(matches!(off(&c), OffReason::Invalid { variable: v, .. } if *v == variable), "{variable}={value}: {:?}", c.account);
        }
        // The site still starts: the router builds with the accounts off.
        let c = with(&account_vars(), &[], &[("AUTH_RATE_BURST", "0")]);
        let _ = crate::app::router(&c, None);
    }

    #[test]
    fn the_rate_limit_can_be_changed_and_the_fake_provider_asked_for() {
        let c = with(&account_vars(), &[], &[("AUTH_RATE_BURST", "1000"), ("AUTH_RATE_PERIOD_SECS", "1"), ("FAKE_PROVIDER", "1")]);
        let AccountSide::On(a) = &c.account else { panic!() };
        assert_eq!((a.rate.period_secs, a.rate.burst, a.fake_provider), (1, 1000, true));
    }
}
