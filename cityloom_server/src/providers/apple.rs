//! Apple wants a client secret that is a JWT signed (ES256) with the team's key. It is signed here with `p256`
//! directly: a JWT library's `rust_crypto` backend would bring `rsa` into the release build for no use.
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use time::{Duration, OffsetDateTime};

pub const SECRET_LIFETIME: Duration = Duration::hours(1);
pub const RENEW_BEFORE: Duration = Duration::minutes(10);
const AUDIENCE: &str = "https://appleid.apple.com";

pub struct AppleSecret {
    team_id: String,
    key_id: String,
    client_id: String,
    key: SigningKey,
    cached: Mutex<Option<(String, OffsetDateTime)>>, // the JWT and when it expires
}

impl AppleSecret {
    pub fn new(team_id: &str, key_id: &str, client_id: &str, private_key_pem: &str) -> Result<AppleSecret, &'static str> {
        let key = SigningKey::from_pkcs8_pem(private_key_pem).map_err(|_| "the Apple private key is not a usable EC key")?;
        Ok(AppleSecret { team_id: team_id.into(), key_id: key_id.into(), client_id: client_id.into(), key, cached: Mutex::new(None) })
    }

    pub fn current(&self, now: OffsetDateTime) -> String {
        let mut cached = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((jwt, expires)) = cached.as_ref()
            && now < *expires - RENEW_BEFORE
        {
            return jwt.clone();
        }
        let expires = now + SECRET_LIFETIME;
        let header = serde_json::json!({ "alg": "ES256", "kid": self.key_id, "typ": "JWT" });
        let claims =
            serde_json::json!({ "iss": self.team_id, "sub": self.client_id, "aud": AUDIENCE, "iat": now.unix_timestamp(), "exp": expires.unix_timestamp() });
        let part = |v: &serde_json::Value| URL_SAFE_NO_PAD.encode(v.to_string());
        let signing_input = format!("{}.{}", part(&header), part(&claims));
        let signature: Signature = self.key.sign(signing_input.as_bytes());
        let jwt = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()));
        *cached = Some((jwt.clone(), expires));
        jwt
    }
}

/// The name Apple sends once, in the `user` form field of the first sign-in.
pub fn first_time_name(user_field: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(user_field).ok()?;
    let name = value.get("name")?.as_object()?;
    let part = |k: &str| name.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty());
    let joined = [part("firstName"), part("lastName")].into_iter().flatten().collect::<Vec<_>>().join(" ");
    (!joined.is_empty()).then_some(joined)
}

/// A freshly made key in PKCS#8 PEM, for tests that need a usable Apple key.
#[cfg(any(test, feature = "test-provider"))]
pub(crate) fn test_key_pem() -> String {
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::{EncodePrivateKey, LineEnding};
    p256::SecretKey::generate().to_pkcs8_pem(LineEnding::LF).expect("a key encodes").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::pkcs8::{EncodePublicKey, LineEnding};
    use time::OffsetDateTime;

    fn keys() -> (String, String) {
        use p256::elliptic_curve::Generate;
        use p256::pkcs8::EncodePrivateKey;
        let secret = p256::SecretKey::generate();
        (secret.to_pkcs8_pem(LineEnding::LF).unwrap().to_string(), secret.public_key().to_public_key_pem(LineEnding::LF).unwrap())
    }

    fn at(s: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000 + s).unwrap()
    }

    fn decode(jwt: &str, public_pem: &str) -> (jsonwebtoken::Header, serde_json::Value) {
        let mut v = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
        v.set_audience(&["https://appleid.apple.com"]);
        v.validate_exp = false;
        let key = jsonwebtoken::DecodingKey::from_ec_pem(public_pem.as_bytes()).unwrap();
        (jsonwebtoken::decode_header(jwt).unwrap(), jsonwebtoken::decode::<serde_json::Value>(jwt, &key, &v).unwrap().claims)
    }

    #[test]
    fn the_client_secret_is_an_es256_jwt_with_apples_claims() {
        let (private, public) = keys();
        let s = AppleSecret::new("TEAM123456", "KEY1234567", "app.cityloom.web", &private).unwrap();
        let (header, claims) = decode(&s.current(at(0)), &public);
        assert_eq!((header.alg, header.kid.as_deref()), (jsonwebtoken::Algorithm::ES256, Some("KEY1234567")));
        assert_eq!(claims["iss"], "TEAM123456");
        assert_eq!(claims["sub"], "app.cityloom.web");
        assert_eq!(claims["aud"], "https://appleid.apple.com");
        assert_eq!(claims["iat"], 1_800_000_000i64);
        assert_eq!(claims["exp"], 1_800_000_000i64 + SECRET_LIFETIME.whole_seconds());
    }

    #[test]
    fn it_is_reused_until_ten_minutes_before_it_expires_and_then_signed_again() {
        let (private, public) = keys();
        let s = AppleSecret::new("T", "K", "c", &private).unwrap();
        let first = s.current(at(0));
        assert_eq!(s.current(at(60)), first);
        assert_eq!(s.current(at(SECRET_LIFETIME.whole_seconds() - RENEW_BEFORE.whole_seconds() - 1)), first);
        let renewed = s.current(at(SECRET_LIFETIME.whole_seconds() - RENEW_BEFORE.whole_seconds()));
        assert_ne!(renewed, first);
        let (_, claims) = decode(&renewed, &public);
        assert_eq!(claims["iat"], 1_800_000_000i64 + SECRET_LIFETIME.whole_seconds() - RENEW_BEFORE.whole_seconds());
    }

    #[test]
    fn a_bad_key_is_refused_without_echoing_it() {
        let e = AppleSecret::new("T", "K", "c", "-----BEGIN PRIVATE KEY-----\nSECRETBITS\n-----END PRIVATE KEY-----").err().unwrap();
        assert!(!e.contains("SECRETBITS"));
    }

    #[test]
    fn the_first_time_name_is_the_first_and_last_name_when_the_field_is_readable() {
        assert_eq!(first_time_name(r#"{"name":{"firstName":"Ada","lastName":"Lovelace"},"email":"a@b.c"}"#).as_deref(), Some("Ada Lovelace"));
        assert_eq!(first_time_name(r#"{"name":{"firstName":"Ada"}}"#).as_deref(), Some("Ada"));
        assert_eq!(first_time_name(r#"{"name":{"lastName":"Lovelace"}}"#).as_deref(), Some("Lovelace"));
        for unreadable in ["", "not json", "{}", r#"{"name":{}}"#, r#"{"name":"Ada"}"#, r#"{"name":{"firstName":"  ","lastName":""}}"#, "[1]"] {
            assert_eq!(first_time_name(unreadable), None, "{unreadable:?}");
        }
    }
}
