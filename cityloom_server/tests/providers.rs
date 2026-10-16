#![cfg(feature = "test-provider")]
mod support;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, header};
use cityloom_server::clock::FixedClock;
use cityloom_server::fake_provider::{self, FakeLogin, LOGIN_COOKIE};
use cityloom_server::flow::FlowState;
use cityloom_server::providers::*;
use http_body_util::BodyExt;
use support::*;
use tower::ServiceExt;

fn fake_spec(form_post: bool) -> ProviderSpec {
    ProviderSpec {
        slug: if form_post { "fake-apple" } else { "fake" }.into(),
        name: "Fake",
        kind: if form_post { Kind::Apple } else { Kind::Standard },
        issuer: ISSUER.into(),
        scopes: vec!["email", "profile"],
        client_id: "cid".into(),
        secret: ClientSecret::Static("secret".into()),
        form_post,
    }
}

fn redirect() -> url::Url {
    "http://127.0.0.1:8137/auth/fake/callback".parse().unwrap()
}

/// Starts a flow and plays the browser through the fake provider's authorize page; gives the flow and the code.
async fn sign_in_as(client: &InProcess, spec: &ProviderSpec, login: &FakeLogin) -> (FlowState, String) {
    let started = begin(spec, &redirect(), client).await.unwrap();
    let path_and_query = format!("{}?{}", started.url.path().trim_start_matches("/fake"), started.url.query().unwrap());
    let res = client
        .0
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/fake{path_and_query}"))
                .header(header::COOKIE, format!("{LOGIN_COOKIE}={}", login.cookie_value()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // A form_post provider answers with a page that posts the fields; the others redirect with them in the query.
    let code = if spec.form_post {
        let html = String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
        fake_provider::parse_form_post(&html).unwrap().1.into_iter().find(|(k, _)| k == "code").unwrap().1
    } else {
        let location = res.headers()[header::LOCATION].to_str().unwrap().to_string();
        url::Url::parse(&location).unwrap().query_pairs().find(|(k, _)| k == "code").unwrap().1.into_owned()
    };
    let flow = FlowState {
        provider: spec.slug.clone(),
        state: started.state,
        nonce: started.nonce,
        pkce_verifier: started.pkce_verifier,
        return_path: "/".into(),
        link_user: None,
        started_at: t0().unix_timestamp(),
    };
    (flow, code)
}

fn client() -> InProcess {
    InProcess::new(Arc::new(FixedClock::new(t0())))
}

#[tokio::test]
async fn begin_builds_an_authorization_url_with_pkce_state_nonce_and_the_scopes() {
    let c = client();
    let s = begin(&fake_spec(false), &redirect(), &c).await.unwrap();
    let q: std::collections::HashMap<_, _> = s.url.query_pairs().into_owned().collect();
    assert_eq!(s.url.path(), "/fake/authorize");
    assert_eq!((q["response_type"].as_str(), q["client_id"].as_str(), q["code_challenge_method"].as_str()), ("code", "cid", "S256"));
    assert_eq!(q["redirect_uri"], redirect().as_str());
    assert_eq!((q["state"].as_str(), q["nonce"].as_str()), (s.state.as_str(), s.nonce.as_str()));
    assert!(q["scope"].split(' ').any(|x| x == "openid") && q["scope"].contains("email") && q["scope"].contains("profile"));
    assert!(!q.contains_key("response_mode"));
    assert!(!s.pkce_verifier.is_empty() && s.state != s.nonce);
    let apple = begin(&fake_spec(true), &redirect(), &c).await.unwrap();
    assert!(apple.url.query_pairs().any(|(k, v)| k == "response_mode" && v == "form_post"));
}

#[tokio::test]
async fn finish_trades_the_code_for_the_verified_claims() {
    let c = client();
    let spec = fake_spec(false);
    let login = FakeLogin {
        sub: Some("sub-9".into()),
        name: Some("Ada".into()),
        email: Some("ada@example.org".into()),
        preferred_username: Some("adaosm".into()),
        ..Default::default()
    };
    let (flow, code) = sign_in_as(&c, &spec, &login).await;
    let claims = finish(&spec, &redirect(), &c, &flow, &code, None, t0()).await.unwrap();
    assert_eq!(
        (claims.provider.as_str(), claims.sub.as_str(), claims.name.as_deref(), claims.label.as_deref()),
        ("fake", "sub-9", Some("Ada"), Some("ada@example.org"))
    );
}

#[tokio::test]
async fn finish_hands_over_apples_first_time_name() {
    let c = client();
    let spec = fake_spec(true);
    let (flow, code) = sign_in_as(&c, &spec, &FakeLogin::default()).await;
    let user = r#"{"name":{"firstName":"Ada","lastName":"Lovelace"}}"#;
    let claims = finish(&spec, &redirect(), &c, &flow, &code, Some(user), t0()).await.unwrap();
    assert_eq!((claims.name, claims.first_time_name.as_deref()), (None, Some("Ada Lovelace")));
}

#[tokio::test]
async fn a_token_for_another_nonce_or_audience_is_refused() {
    let c = client();
    let spec = fake_spec(false);
    for login in [FakeLogin { nonce: Some("not-yours".into()), ..Default::default() }, FakeLogin { aud: Some("someone-else".into()), ..Default::default() }] {
        let (flow, code) = sign_in_as(&c, &spec, &login).await;
        assert!(matches!(finish(&spec, &redirect(), &c, &flow, &code, None, t0()).await, Err(ProviderError::Token(_))));
    }
}

#[tokio::test]
async fn a_code_that_was_not_issued_or_a_wrong_verifier_fails_the_exchange() {
    let c = client();
    let spec = fake_spec(false);
    let (mut flow, code) = sign_in_as(&c, &spec, &FakeLogin::default()).await;
    assert!(matches!(finish(&spec, &redirect(), &c, &flow, "made-up", None, t0()).await, Err(ProviderError::Exchange)));
    flow.pkce_verifier = "another-verifier".into();
    assert!(matches!(finish(&spec, &redirect(), &c, &flow, &code, None, t0()).await, Err(ProviderError::Exchange)));
}

#[tokio::test]
async fn clock_skew_of_a_minute_is_tolerated_and_more_is_not() {
    let c = client();
    let spec = fake_spec(false);
    for (iat, exp, ok) in [(30, 0, true), (59, 0, true), (90, 0, false), (0, -3630, true), (0, -3700, false)] {
        let (flow, code) = sign_in_as(&c, &spec, &FakeLogin { iat_offset: iat, exp_offset: exp, ..Default::default() }).await;
        let r = finish(&spec, &redirect(), &c, &flow, &code, None, t0()).await;
        assert_eq!(r.is_ok(), ok, "iat {iat:+} exp {exp:+}: {:?}", r.err());
    }
}

#[tokio::test]
async fn a_provider_that_is_down_is_unreachable() {
    assert!(matches!(begin(&fake_spec(false), &redirect(), &Down).await, Err(ProviderError::Unreachable)));
}

#[tokio::test]
async fn apple_sends_its_credentials_in_the_body_and_basic_is_refused_by_the_apple_like_fake() {
    let c = client();
    // Kind::Apple: the fake (form_post mode) answers invalid_client to HTTP Basic, so this passing proves the body.
    let spec = fake_spec(true);
    let (flow, code) = sign_in_as(&c, &spec, &FakeLogin::default()).await;
    assert!(finish(&spec, &redirect(), &c, &flow, &code, None, t0()).await.is_ok());
    // The same provider treated as a standard one sends Basic, which the fake refuses.
    let basic = ProviderSpec { kind: Kind::Standard, ..fake_spec(true) };
    let (flow, code) = sign_in_as(&c, &basic, &FakeLogin::default()).await;
    assert!(matches!(finish(&basic, &redirect(), &c, &flow, &code, None, t0()).await, Err(ProviderError::Exchange)));
}
