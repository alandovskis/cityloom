//! The four providers behind one OpenID Connect client: Authorization Code with PKCE (S256), `state` and `nonce`.
use async_trait::async_trait;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreJsonWebKeySet, CoreProviderMetadata};
use openidconnect::{
    AuthType, AuthorizationCode, ClaimsVerificationError, ClientId, ClientSecret as OidcSecret, CsrfToken, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse,
};
use time::OffsetDateTime;

use crate::accounts::Claims;
use crate::config::{AccountConfig, ProviderCreds, ProviderSecret};
use crate::flow::FlowState;

pub mod apple;
pub mod microsoft;

pub const SKEW_SECS: i64 = 60;

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String>;
}

pub struct ReqwestClient(reqwest::Client);

impl ReqwestClient {
    pub fn new() -> ReqwestClient {
        ReqwestClient(
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("a client builds"),
        )
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String> {
        let (parts, body) = request.into_parts();
        let res =
            self.0.request(parts.method, parts.uri.to_string()).headers(parts.headers).body(body).send().await.map_err(|e| e.without_url().to_string())?;
        let mut out = http::Response::builder().status(res.status());
        *out.headers_mut().expect("builder is fine") = res.headers().clone();
        let bytes = res.bytes().await.map_err(|e| e.without_url().to_string())?;
        out.body(bytes.to_vec()).map_err(|e| e.to_string())
    }
}

/// Polls a future that has nothing to wait for, once.
fn run_to_the_end<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => unreachable!("the stand-in clients answer at once"),
    }
}

/// What the stand-in clients answer when they are not to send anything.
#[derive(Debug, thiserror::Error)]
#[error("the request failed")]
struct AdapterError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Microsoft,
    Apple,
}

pub enum ClientSecret {
    Static(String),
    Apple(apple::AppleSecret),
}

impl ClientSecret {
    fn value(&self, now: OffsetDateTime) -> String {
        match self {
            ClientSecret::Static(s) => s.clone(),
            ClientSecret::Apple(a) => a.current(now),
        }
    }
}

pub struct ProviderSpec {
    pub slug: String,
    pub name: &'static str,
    pub kind: Kind,
    pub issuer: String,
    pub scopes: Vec<&'static str>,
    pub client_id: String,
    pub secret: ClientSecret,
    pub form_post: bool,
}

#[derive(Default)]
pub struct Providers {
    specs: Vec<ProviderSpec>,
}

impl Providers {
    pub fn from_config(config: &AccountConfig) -> Providers {
        Providers::from_creds(&config.providers)
    }

    fn from_creds(all: &[ProviderCreds]) -> Providers {
        let mut specs = Vec::new();
        for creds in all {
            let secret = match &creds.secret {
                ProviderSecret::Static(s) => ClientSecret::Static(s.clone()),
                ProviderSecret::Apple { team_id, key_id, private_key_pem } => match apple::AppleSecret::new(team_id, key_id, &creds.client_id, private_key_pem)
                {
                    Ok(a) => ClientSecret::Apple(a),
                    Err(_) => {
                        tracing::warn!("APPLE_PRIVATE_KEY is not a usable key");
                        continue;
                    }
                },
            };
            let (name, kind, issuer, scopes, form_post) = match creds.slug {
                "osm" => ("OpenStreetMap", Kind::Standard, "https://www.openstreetmap.org", vec![], false),
                "google" => ("Google", Kind::Standard, "https://accounts.google.com", vec!["email", "profile"], false),
                "microsoft" => ("Microsoft", Kind::Microsoft, "https://login.microsoftonline.com/common/v2.0", vec!["email", "profile"], false),
                "apple" => ("Apple", Kind::Apple, "https://appleid.apple.com", vec!["name", "email"], true),
                _ => continue,
            };
            specs.push(ProviderSpec {
                slug: creds.slug.to_string(),
                name,
                kind,
                issuer: issuer.into(),
                scopes,
                client_id: creds.client_id.clone(),
                secret,
                form_post,
            });
        }
        Providers { specs }
    }

    /// The named real providers with dummy credentials, in the fixed order (osm, google, microsoft, apple).
    #[cfg(any(test, feature = "test-provider"))]
    pub fn from_config_for_test(slugs: &[&str]) -> Providers {
        let pem = apple::test_key_pem();
        let creds: Vec<ProviderCreds> = ["osm", "google", "microsoft", "apple"]
            .into_iter()
            .filter(|s| slugs.contains(s))
            .map(|slug| ProviderCreds {
                slug,
                client_id: format!("{slug}-id"),
                secret: if slug == "apple" {
                    ProviderSecret::Apple { team_id: "T".into(), key_id: "K".into(), private_key_pem: pem.clone() }
                } else {
                    ProviderSecret::Static("s".into())
                },
            })
            .collect();
        Providers::from_creds(&creds)
    }

    pub fn with(mut self, spec: ProviderSpec) -> Providers {
        self.specs.push(spec);
        self
    }

    pub fn get(&self, slug: &str) -> Option<&ProviderSpec> {
        self.specs.iter().find(|s| s.slug == slug)
    }

    pub fn list(&self) -> Vec<(String, &'static str)> {
        self.specs.iter().map(|s| (s.slug.clone(), s.name)).collect()
    }
}

pub struct Started {
    pub url: url::Url,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("the provider could not be reached")]
    Unreachable,
    #[error("the provider's discovery document was refused")]
    Discovery,
    #[error("the code exchange failed")]
    Exchange,
    #[error("the ID token was refused: {0}")]
    Token(&'static str),
}

async fn get(http: &dyn HttpClient, url: &str) -> Result<Vec<u8>, ProviderError> {
    let request = http::Request::get(url).header("accept", "application/json").body(Vec::new()).map_err(|_| ProviderError::Discovery)?;
    let response = http.send(request).await.map_err(|_| ProviderError::Unreachable)?;
    if response.status() != http::StatusCode::OK {
        return Err(ProviderError::Discovery);
    }
    Ok(response.into_body())
}

/// The provider's discovery document and keys. One path for every provider; Microsoft's template issuer is checked
/// here and replaced by the configured `common` one, which is what openidconnect then holds the token's `iss` to
/// (and is switched off for Microsoft in `finish`, where the token's own tenant is used instead).
async fn metadata(spec: &ProviderSpec, http: &dyn HttpClient) -> Result<CoreProviderMetadata, ProviderError> {
    let body = get(http, &format!("{}/.well-known/openid-configuration", spec.issuer.trim_end_matches('/'))).await?;
    let mut doc: serde_json::Value = serde_json::from_slice(&body).map_err(|_| ProviderError::Discovery)?;
    let expected = if spec.kind == Kind::Microsoft { microsoft::TEMPLATE } else { spec.issuer.as_str() };
    if doc.get("issuer").and_then(|i| i.as_str()) != Some(expected) {
        return Err(ProviderError::Discovery);
    }
    doc["issuer"] = serde_json::Value::String(spec.issuer.clone());
    let metadata: CoreProviderMetadata = serde_json::from_value(doc).map_err(|_| ProviderError::Discovery)?;
    let keys = get(http, metadata.jwks_uri().as_str()).await?;
    let keys: CoreJsonWebKeySet = serde_json::from_slice(&keys).map_err(|_| ProviderError::Discovery)?;
    Ok(metadata.set_jwks(keys))
}

pub async fn begin(spec: &ProviderSpec, redirect_uri: &url::Url, http: &dyn HttpClient) -> Result<Started, ProviderError> {
    let client = CoreClient::from_provider_metadata(metadata(spec, http).await?, ClientId::new(spec.client_id.clone()), None)
        .set_redirect_uri(RedirectUrl::from_url(redirect_uri.clone()));
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut request = client.authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random).set_pkce_challenge(challenge);
    for scope in &spec.scopes {
        request = request.add_scope(Scope::new((*scope).to_string()));
    }
    if spec.form_post {
        request = request.add_extra_param("response_mode", "form_post");
    }
    let (url, state, nonce) = request.url();
    Ok(Started { url, state: state.secret().clone(), nonce: nonce.secret().clone(), pkce_verifier: verifier.secret().clone() })
}

fn token_error(e: ClaimsVerificationError) -> ProviderError {
    ProviderError::Token(match e {
        ClaimsVerificationError::Expired(_) => "expired",
        ClaimsVerificationError::InvalidAudience(_) => "audience",
        ClaimsVerificationError::InvalidIssuer(_) => "issuer",
        ClaimsVerificationError::InvalidNonce(_) => "nonce",
        ClaimsVerificationError::SignatureVerification(_) => "signature",
        _ => "invalid",
    })
}

/// The `tid` claim of an ID token whose signature was already verified.
fn tenant_of(id_token: &CoreIdToken) -> Option<String> {
    use base64::Engine;
    let jwt = serde_json::to_value(id_token).ok()?.as_str()?.to_string();
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(jwt.split('.').nth(1)?).ok()?;
    serde_json::from_slice::<serde_json::Value>(&payload).ok()?.get("tid")?.as_str().map(str::to_string)
}

pub async fn finish(
    spec: &ProviderSpec,
    redirect_uri: &url::Url,
    http: &dyn HttpClient,
    flow: &FlowState,
    code: &str,
    user_field: Option<&str>,
    now: OffsetDateTime,
) -> Result<Claims, ProviderError> {
    let mut client =
        CoreClient::from_provider_metadata(metadata(spec, http).await?, ClientId::new(spec.client_id.clone()), Some(OidcSecret::new(spec.secret.value(now))))
            .set_redirect_uri(RedirectUrl::from_url(redirect_uri.clone()));
    if spec.kind == Kind::Apple {
        // Apple's token endpoint takes `client_secret_post` only; openidconnect's default is HTTP Basic.
        client = client.set_auth_type(AuthType::RequestBody);
    }
    // The token endpoint is called by hand, between two runs of openidconnect's request that never wait: the first
    // only hands over the request it would send, the second reads the answer. (Awaiting `request_async(&client)` with
    // a client of our own makes rustc fail to prove a handler that awaits `finish` `Send`: "higher-ranked lifetime
    // error". Nothing else of openidconnect is awaited.)
    let exchange = || {
        client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map(|request| request.set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier.clone())))
            .map_err(|_| ProviderError::Exchange)
    };
    let request = {
        let asked = std::cell::RefCell::new(None);
        let _ = run_to_the_end(exchange()?.request_async(&|request| {
            *asked.borrow_mut() = Some(request);
            std::future::ready(Err::<openidconnect::HttpResponse, _>(AdapterError))
        }));
        asked.into_inner().ok_or(ProviderError::Exchange)?
    };
    let response = http.send(request).await.map_err(|_| ProviderError::Unreachable)?;
    let token =
        run_to_the_end(exchange()?.request_async(&|_| std::future::ready(Ok::<_, AdapterError>(response.clone())))).map_err(|_| ProviderError::Exchange)?;
    let id_token = token.id_token().ok_or(ProviderError::Token("missing"))?;
    let skew = chrono::Duration::seconds(SKEW_SECS);
    let at = chrono::DateTime::from_timestamp(now.unix_timestamp(), 0).unwrap_or_default();
    let mut verifier = client
        .id_token_verifier()
        .set_time_fn(move || at - skew)
        .set_issue_time_verifier_fn(move |iat| if iat <= at + skew { Ok(()) } else { Err("issued in the future".to_string()) });
    if spec.kind == Kind::Microsoft {
        verifier = verifier.require_issuer_match(false);
    }
    let claims = id_token.claims(&verifier, &Nonce::new(flow.nonce.clone())).map_err(token_error)?;
    if spec.kind == Kind::Microsoft {
        microsoft::check_issuer(claims.issuer().as_str(), tenant_of(id_token).as_deref()).map_err(ProviderError::Token)?;
    }
    Ok(build_claims(
        spec,
        claims.subject().as_str(),
        claims.name().and_then(|n| n.get(None)).map(|n| n.as_str()),
        claims.preferred_username().map(|u| u.as_str()),
        claims.email().map(|e| e.as_str()),
        user_field,
    ))
}

pub fn build_claims(
    spec: &ProviderSpec,
    sub: &str,
    name: Option<&str>,
    preferred_username: Option<&str>,
    email: Option<&str>,
    user_field: Option<&str>,
) -> Claims {
    let own = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    Claims {
        provider: spec.slug.clone(),
        provider_name: spec.name.to_string(),
        sub: sub.to_string(),
        name: own(name),
        preferred_username: own(preferred_username),
        first_time_name: if spec.form_post { user_field.and_then(apple::first_time_name) } else { None },
        // Law 25: the email only where it is the label; OpenStreetMap is told apart by its username.
        label: if spec.slug == "osm" { own(preferred_username) } else { own(email) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(slug: &str, kind: Kind) -> ProviderSpec {
        ProviderSpec {
            slug: slug.into(),
            name: "Name",
            kind,
            issuer: "https://i".into(),
            scopes: vec![],
            client_id: "c".into(),
            secret: ClientSecret::Static("s".into()),
            form_post: kind == Kind::Apple,
        }
    }

    #[test]
    fn the_label_is_the_username_for_openstreetmap_and_the_email_for_the_others() {
        let c = build_claims(&spec("osm", Kind::Standard), "1", None, Some("mapper"), Some("m@example.org"), None);
        assert_eq!(c.label.as_deref(), Some("mapper"));
        let c = build_claims(&spec("google", Kind::Standard), "1", Some("Ada"), None, Some("a@example.org"), None);
        assert_eq!((c.label.as_deref(), c.name.as_deref(), c.provider.as_str(), c.sub.as_str()), (Some("a@example.org"), Some("Ada"), "google", "1"));
        let c = build_claims(&spec("google", Kind::Standard), "1", None, None, None, None);
        assert_eq!(c.label, None);
    }

    #[test]
    fn the_apple_user_field_gives_the_first_time_name_only_for_a_form_post_provider() {
        let user = r#"{"name":{"firstName":"Ada","lastName":"Lovelace"}}"#;
        let c = build_claims(&spec("apple", Kind::Apple), "a", None, None, Some("relay@privaterelay.example"), Some(user));
        assert_eq!(c.first_time_name.as_deref(), Some("Ada Lovelace"));
        let c = build_claims(&spec("google", Kind::Standard), "a", None, None, None, Some(user));
        assert_eq!(c.first_time_name, None, "only Apple sends it");
        let c = build_claims(&spec("apple", Kind::Apple), "a", None, None, None, Some("not json"));
        assert_eq!(c.first_time_name, None, "an unreadable field is ignored, not an error");
    }

    #[test]
    fn providers_list_only_what_is_configured_in_order() {
        let p = Providers::from_config_for_test(&["google", "osm"]);
        assert_eq!(p.list().into_iter().map(|(s, _)| s).collect::<Vec<_>>(), ["osm", "google"]);
        assert!(p.get("apple").is_none());
    }
}
