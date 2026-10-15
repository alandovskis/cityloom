//! An OpenID provider for tests. Compiled only with the `test-provider` feature.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use p256::elliptic_curve::Generate;
use p256::pkcs8::{EncodePrivateKey, LineEnding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clock::Clock;
use crate::crypto::random_bytes;

pub const LOGIN_COOKIE: &str = "fake-login";
const LIFETIME_SECS: i64 = 3600;
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FakeLogin {
    pub sub: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub tid: Option<String>,
    pub deny: bool,
    pub iat_offset: i64,
    pub exp_offset: i64,
    pub aud: Option<String>,
    pub nonce: Option<String>,
    pub first_time_user: Option<serde_json::Value>,
}

impl FakeLogin {
    pub fn cookie_value(&self) -> String {
        percent_encoding::utf8_percent_encode(&serde_json::to_string(self).unwrap(), percent_encoding::NON_ALPHANUMERIC).to_string()
    }

    fn from_headers(headers: &HeaderMap) -> FakeLogin {
        let raw = headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|p| p.trim().strip_prefix(&format!("{LOGIN_COOKIE}=")))
            .next();
        raw.and_then(|r| percent_encoding::percent_decode_str(r).decode_utf8().ok()).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
    }
}

struct Code {
    login: FakeLogin,
    client_id: String,
    nonce: String,
    redirect_uri: String,
    challenge: String,
}

pub struct FakeState {
    issuer: String,
    clock: Arc<dyn Clock>,
    signing: jsonwebtoken::EncodingKey,
    jwk: serde_json::Value,
    codes: Mutex<HashMap<String, Code>>,
}

impl FakeState {
    pub fn new(issuer: String, clock: Arc<dyn Clock>) -> Arc<FakeState> {
        let secret = p256::SecretKey::generate();
        let pem = secret.to_pkcs8_pem(LineEnding::LF).expect("a P-256 key encodes");
        let point = secret.public_key().to_sec1_bytes(); // 0x04 ‖ x ‖ y
        let jwk = serde_json::json!({ "kty": "EC", "crv": "P-256", "alg": "ES256", "use": "sig", "kid": "fake-1", "x": B64.encode(&point[1..33]), "y": B64.encode(&point[33..65]) });
        Arc::new(FakeState {
            issuer,
            clock,
            signing: jsonwebtoken::EncodingKey::from_ec_pem(pem.as_bytes()).expect("the PEM is ours"),
            jwk,
            codes: Mutex::new(HashMap::new()),
        })
    }

    fn id_token(&self, code: &Code) -> String {
        let l = &code.login;
        let now = self.clock.now().unix_timestamp();
        let mut claims = serde_json::json!({
            "iss": self.issuer,
            "sub": l.sub.clone().unwrap_or_else(|| "fake-user".into()),
            "aud": l.aud.clone().unwrap_or_else(|| code.client_id.clone()),
            "iat": now + l.iat_offset,
            "exp": now + LIFETIME_SECS + l.exp_offset,
            "nonce": l.nonce.clone().unwrap_or_else(|| code.nonce.clone()),
        });
        for (k, v) in [("name", &l.name), ("email", &l.email), ("preferred_username", &l.preferred_username), ("tid", &l.tid)] {
            if let Some(v) = v {
                claims[k] = serde_json::json!(v);
            }
        }
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some("fake-1".into());
        jsonwebtoken::encode(&header, &claims, &self.signing).expect("claims encode")
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The action and the hidden fields of the page `form_post` mode answers with.
pub fn parse_form_post(html: &str) -> Option<(String, Vec<(String, String)>)> {
    let action = unescape(html.split("<form method=\"post\" action=\"").nth(1)?.split('"').next()?);
    let fields = html
        .split("<input type=\"hidden\" name=\"")
        .skip(1)
        .filter_map(|part| {
            let (name, rest) = part.split_once("\" value=\"")?;
            Some((unescape(name), unescape(rest.split('"').next()?)))
        })
        .collect();
    Some((action, fields))
}

async fn discovery(State(s): State<Arc<FakeState>>) -> Json<serde_json::Value> {
    let i = &s.issuer;
    Json(serde_json::json!({
        "issuer": i,
        "authorization_endpoint": format!("{i}/authorize"),
        "token_endpoint": format!("{i}/token"),
        "jwks_uri": format!("{i}/jwks"),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query", "form_post"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["ES256"],
        "scopes_supported": ["openid", "email", "profile", "name"],
        "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

async fn jwks(State(s): State<Arc<FakeState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "keys": [s.jwk.clone()] }))
}

#[derive(Deserialize)]
struct AuthorizeQuery {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    response_mode: Option<String>,
}

async fn authorize(State(s): State<Arc<FakeState>>, headers: HeaderMap, Query(q): Query<AuthorizeQuery>) -> Response {
    let login = FakeLogin::from_headers(&headers);
    let state = q.state.clone().unwrap_or_default();
    let form_post = q.response_mode.as_deref() == Some("form_post");
    let mut fields: Vec<(&str, String)> = Vec::new();
    if login.deny {
        fields.push(("error", "access_denied".into()));
    } else {
        let code = B64.encode(random_bytes::<18>());
        if let Some(user) = &login.first_time_user {
            fields.push(("user", user.to_string()));
        }
        fields.push(("code", code.clone()));
        s.codes.lock().unwrap().insert(
            code,
            Code {
                login,
                client_id: q.client_id.clone(),
                nonce: q.nonce.clone().unwrap_or_default(),
                redirect_uri: q.redirect_uri.clone(),
                challenge: q.code_challenge.clone().unwrap_or_default(),
            },
        );
    }
    fields.push(("state", state));
    if form_post {
        let inputs: String = fields.iter().map(|(k, v)| format!("<input type=\"hidden\" name=\"{}\" value=\"{}\">", escape(k), escape(v))).collect();
        return Html(format!(
            "<!doctype html><html><body><form method=\"post\" action=\"{}\">{inputs}</form><script>document.forms[0].submit()</script></body></html>",
            escape(&q.redirect_uri)
        ))
        .into_response();
    }
    let mut url = url::Url::parse(&q.redirect_uri).unwrap_or_else(|_| url::Url::parse("http://invalid/").unwrap());
    // The `user` field exists only in form_post mode.
    url.query_pairs_mut().extend_pairs(fields.iter().filter(|(k, _)| *k != "user").map(|(k, v)| (*k, v.as_str())));
    Redirect::to(url.as_str()).into_response()
}

#[derive(Deserialize)]
struct TokenForm {
    code: String,
    redirect_uri: String,
    code_verifier: Option<String>,
}

async fn token(State(s): State<Arc<FakeState>>, Form(f): Form<TokenForm>) -> Response {
    let bad = || (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "invalid_grant" }))).into_response();
    // Taken out whatever happens: a guess gets one go.
    let Some(code) = s.codes.lock().unwrap().remove(&f.code) else { return bad() };
    let verifier_ok = f.code_verifier.as_deref().is_some_and(|v| B64.encode(Sha256::digest(v.as_bytes())) == code.challenge);
    if f.redirect_uri != code.redirect_uri || !verifier_ok {
        return bad();
    }
    Json(serde_json::json!({ "access_token": B64.encode(random_bytes::<16>()), "token_type": "Bearer", "expires_in": LIFETIME_SECS, "id_token": s.id_token(&code) })).into_response()
}

pub fn router(state: Arc<FakeState>) -> Router {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use base64::Engine;
    use http_body_util::BodyExt;
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;

    const NOW: i64 = 1_800_000_000;

    fn app() -> Router {
        router(FakeState::new("http://127.0.0.1:8137/fake".into(), Arc::new(FixedClock::new(time::OffsetDateTime::from_unix_timestamp(NOW).unwrap()))))
    }

    fn challenge(verifier: &str) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
    }

    async fn call(app: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
        let res = app.clone().oneshot(req).await.unwrap();
        let (p, b) = res.into_parts();
        (p.status, p.headers, String::from_utf8(b.collect().await.unwrap().to_bytes().to_vec()).unwrap())
    }

    fn authorize(login: &FakeLogin, extra: &str) -> Request<Body> {
        Request::builder()
            .uri(format!("/authorize?response_type=code&client_id=cid&redirect_uri=http%3A%2F%2Fapp%2Fcb&state=st1&nonce=n1&scope=openid&code_challenge={}&code_challenge_method=S256{extra}", challenge("ver")))
            .header(header::COOKIE, format!("{LOGIN_COOKIE}={}", login.cookie_value()))
            .body(Body::empty())
            .unwrap()
    }

    fn code_of(location: &str) -> String {
        url::Url::parse(location).unwrap().query_pairs().find(|(k, _)| k == "code").unwrap().1.into_owned()
    }

    async fn exchange(app: &Router, code: &str, verifier: &str) -> (StatusCode, serde_json::Value) {
        let body =
            format!("grant_type=authorization_code&code={code}&redirect_uri=http%3A%2F%2Fapp%2Fcb&code_verifier={verifier}&client_id=cid&client_secret=s");
        let (status, _, text) = call(
            app,
            Request::builder().method("POST").uri("/token").header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(Body::from(body)).unwrap(),
        )
        .await;
        (status, serde_json::from_str(&text).unwrap())
    }

    fn claims_of(id_token: &str, jwks: &serde_json::Value) -> serde_json::Value {
        let k = &jwks["keys"][0];
        let key = jsonwebtoken::DecodingKey::from_ec_components(k["x"].as_str().unwrap(), k["y"].as_str().unwrap()).unwrap();
        let mut v = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
        v.set_audience(&["cid"]);
        v.validate_exp = false;
        v.set_issuer(&["http://127.0.0.1:8137/fake"]);
        jsonwebtoken::decode::<serde_json::Value>(id_token, &key, &v).unwrap().claims
    }

    #[tokio::test]
    async fn discovery_names_the_endpoints_under_the_issuer_and_the_jwks_has_one_ec_key() {
        let a = app();
        let (s, _, text) = call(&a, Request::builder().uri("/.well-known/openid-configuration").body(Body::empty()).unwrap()).await;
        let d: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(s, StatusCode::OK);
        assert_eq!(d["issuer"], "http://127.0.0.1:8137/fake");
        assert_eq!(d["authorization_endpoint"], "http://127.0.0.1:8137/fake/authorize");
        assert_eq!(d["token_endpoint"], "http://127.0.0.1:8137/fake/token");
        assert_eq!(d["jwks_uri"], "http://127.0.0.1:8137/fake/jwks");
        assert_eq!(d["id_token_signing_alg_values_supported"][0], "ES256");
        let (_, _, text) = call(&a, Request::builder().uri("/jwks").body(Body::empty()).unwrap()).await;
        let k: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!((k["keys"].as_array().unwrap().len(), k["keys"][0]["kty"].as_str(), k["keys"][0]["crv"].as_str()), (1, Some("EC"), Some("P-256")));
    }

    #[tokio::test]
    async fn authorize_redirects_back_with_a_code_and_the_state() {
        let a = app();
        let (s, h, _) = call(&a, authorize(&FakeLogin::default(), "")).await;
        assert_eq!(s, StatusCode::SEE_OTHER);
        let loc = h[header::LOCATION].to_str().unwrap();
        assert!(loc.starts_with("http://app/cb?"), "{loc}");
        assert!(url::Url::parse(loc).unwrap().query_pairs().any(|(k, v)| k == "state" && v == "st1"));
        assert!(!code_of(loc).is_empty());
    }

    #[tokio::test]
    async fn a_person_who_says_no_is_sent_back_with_access_denied_and_the_state() {
        let (_, h, _) = call(&app(), authorize(&FakeLogin { deny: true, ..Default::default() }, "")).await;
        let loc = url::Url::parse(h[header::LOCATION].to_str().unwrap()).unwrap();
        let q: std::collections::HashMap<_, _> = loc.query_pairs().into_owned().collect();
        assert_eq!((q["error"].as_str(), q["state"].as_str(), q.contains_key("code")), ("access_denied", "st1", false));
    }

    #[tokio::test]
    async fn the_code_buys_an_id_token_that_the_published_key_verifies() {
        let a = app();
        let login = FakeLogin { sub: Some("sub-1".into()), name: Some("Ada".into()), email: Some("ada@example.org".into()), ..Default::default() };
        let (_, h, _) = call(&a, authorize(&login, "")).await;
        let (s, body) = exchange(&a, &code_of(h[header::LOCATION].to_str().unwrap()), "ver").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body["token_type"], "Bearer");
        let (_, _, jwks) = call(&a, Request::builder().uri("/jwks").body(Body::empty()).unwrap()).await;
        let c = claims_of(body["id_token"].as_str().unwrap(), &serde_json::from_str(&jwks).unwrap());
        assert_eq!(
            (c["iss"].as_str(), c["sub"].as_str(), c["aud"].as_str(), c["nonce"].as_str()),
            (Some("http://127.0.0.1:8137/fake"), Some("sub-1"), Some("cid"), Some("n1"))
        );
        assert_eq!((c["name"].as_str(), c["email"].as_str()), (Some("Ada"), Some("ada@example.org")));
        assert_eq!((c["iat"].as_i64(), c["exp"].as_i64()), (Some(NOW), Some(NOW + 3600)));
    }

    #[tokio::test]
    async fn the_login_can_skew_the_clock_change_the_audience_and_the_nonce() {
        let a = app();
        let login = FakeLogin { iat_offset: 90, exp_offset: -3700, aud: Some("someone-else".into()), nonce: Some("other".into()), ..Default::default() };
        let (_, h, _) = call(&a, authorize(&login, "")).await;
        let (_, body) = exchange(&a, &code_of(h[header::LOCATION].to_str().unwrap()), "ver").await;
        let payload = body["id_token"].as_str().unwrap().split('.').nth(1).unwrap().to_string();
        let c: serde_json::Value = serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).unwrap()).unwrap();
        assert_eq!(
            (c["iat"].as_i64(), c["exp"].as_i64(), c["aud"].as_str(), c["nonce"].as_str()),
            (Some(NOW + 90), Some(NOW - 100), Some("someone-else"), Some("other"))
        );
    }

    #[tokio::test]
    async fn a_code_works_once_and_only_with_its_verifier() {
        let a = app();
        let (_, h, _) = call(&a, authorize(&FakeLogin::default(), "")).await;
        let code = code_of(h[header::LOCATION].to_str().unwrap());
        assert_eq!(exchange(&a, &code, "wrong").await.0, StatusCode::BAD_REQUEST);
        // A failed try spends the code too: a guessed verifier gets one go.
        assert_eq!(exchange(&a, &code, "ver").await.0, StatusCode::BAD_REQUEST);
        let (_, h, _) = call(&a, authorize(&FakeLogin::default(), "")).await;
        let code = code_of(h[header::LOCATION].to_str().unwrap());
        assert_eq!(exchange(&a, &code, "ver").await.0, StatusCode::OK);
        assert_eq!(exchange(&a, &code, "ver").await.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn form_post_mode_answers_with_a_page_that_posts_the_code_state_and_the_first_time_user() {
        let a = app();
        let login = FakeLogin { first_time_user: Some(serde_json::json!({"name": {"firstName": "Ada", "lastName": "Lovelace"}})), ..Default::default() };
        let (s, h, html) = call(&a, authorize(&login, "&response_mode=form_post")).await;
        assert_eq!(s, StatusCode::OK);
        assert!(h[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
        let (action, fields) = parse_form_post(&html).unwrap();
        assert_eq!(action, "http://app/cb");
        let get = |n: &str| fields.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        assert_eq!(get("state").as_deref(), Some("st1"));
        assert!(get("code").is_some());
        let user: serde_json::Value = serde_json::from_str(&get("user").unwrap()).unwrap();
        assert_eq!(user["name"]["firstName"], "Ada");
        let (_, _, html) = call(&a, authorize(&FakeLogin::default(), "&response_mode=form_post")).await;
        assert!(parse_form_post(&html).unwrap().1.iter().all(|(k, _)| k != "user"), "no user field the second time");
    }
}
