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
    let rate =
        RateLimit { period_secs: positive("AUTH_RATE_PERIOD_SECS", DEFAULT_PERIOD_SECS as u32)? as u64, burst: positive("AUTH_RATE_BURST", DEFAULT_BURST)? };
    // An empty value (`.env.example` ships them) is not set.
    let set = |variable: &str| get(variable).filter(|v| !v.trim().is_empty());
    let pair = |slug: &'static str, id: &str, secret: &str| -> Option<ProviderCreds> {
        Some(ProviderCreds { slug, client_id: set(id)?, secret: ProviderSecret::Static(set(secret)?) })
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
        (set("APPLE_CLIENT_ID"), set("APPLE_TEAM_ID"), set("APPLE_KEY_ID"), set("APPLE_PRIVATE_KEY"))
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
mod tests;
