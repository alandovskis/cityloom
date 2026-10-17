//! Keys, sealing and the identity lookup hash. Nothing here is logged or put in an error.
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::{ZeroizeOnDrop, Zeroizing};

pub const SEAL_VERSION: u8 = 1;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    #[error("sealed value is malformed")]
    Format,
    #[error("sealed value did not open")]
    Open,
    #[error("no master key with id {0}")]
    UnknownKeyId(u8),
    #[error("master keys are not usable: {0}")]
    Keys(&'static str),
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("the operating system has randomness");
    bytes
}

#[derive(Clone, ZeroizeOnDrop)]
pub struct Key(pub(crate) [u8; 32]);

impl Key {
    pub fn random() -> Key {
        Key(random_bytes())
    }
    pub fn from_bytes(bytes: [u8; 32]) -> Key {
        Key(bytes)
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(..)")
    }
}

/// The associated data of a sealed field: where it lives, so it opens nowhere else.
pub fn aad(table: &str, column: &str, id: &str) -> String {
    format!("{table}.{column}.{id}")
}

pub fn seal(key: &Key, aad: &str, plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(&key.0).expect("a key is 32 bytes");
    let nonce_bytes: [u8; NONCE_LEN] = random_bytes();
    let body = cipher.encrypt(&Nonce::from(nonce_bytes), Payload { msg: plaintext, aad: aad.as_bytes() }).expect("AES-GCM encrypts any message this small");
    let mut out = Vec::with_capacity(1 + NONCE_LEN + body.len());
    out.push(SEAL_VERSION);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&body); // ciphertext ‖ tag
    out
}

pub fn open(key: &Key, aad: &str, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if sealed.len() < 1 + NONCE_LEN + TAG_LEN || sealed[0] != SEAL_VERSION {
        return Err(CryptoError::Format);
    }
    let cipher = Aes256Gcm::new_from_slice(&key.0).expect("a key is 32 bytes");
    let (nonce, body) = sealed[1..].split_at(NONCE_LEN);
    let nonce = Nonce::try_from(nonce).map_err(|_| CryptoError::Format)?;
    cipher.decrypt(&nonce, Payload { msg: body, aad: aad.as_bytes() }).map(Zeroizing::new).map_err(|_| CryptoError::Open)
}

pub struct MasterKeys {
    keys: Vec<(u8, Key)>,
    current: u8,
}

impl MasterKeys {
    pub fn new(keys: &[(u8, Zeroizing<[u8; 32]>)], current: u8) -> Result<MasterKeys, CryptoError> {
        let mut seen = std::collections::BTreeSet::new();
        if !keys.iter().all(|(id, _)| seen.insert(*id)) {
            return Err(CryptoError::Keys("a key id is used twice"));
        }
        if !seen.contains(&current) {
            return Err(CryptoError::Keys("the current key is not listed"));
        }
        Ok(MasterKeys { keys: keys.iter().map(|(id, k)| (*id, Key(**k))).collect(), current })
    }

    pub fn current_id(&self) -> u8 {
        self.current
    }

    fn get(&self, id: u8) -> Result<&Key, CryptoError> {
        self.keys.iter().find(|(i, _)| *i == id).map(|(_, k)| k).ok_or(CryptoError::UnknownKeyId(id))
    }

    fn wrap_aad(user: uuid::Uuid) -> String {
        aad("users", "wrapped_key", &user.to_string())
    }

    pub fn wrap(&self, user: uuid::Uuid, data_key: &Key) -> Vec<u8> {
        seal(self.get(self.current).expect("the current key is listed"), &Self::wrap_aad(user), &data_key.0)
    }

    pub fn unwrap(&self, key_id: u8, user: uuid::Uuid, wrapped: &[u8]) -> Result<Key, CryptoError> {
        let opened = open(self.get(key_id)?, &Self::wrap_aad(user), wrapped)?;
        let bytes: Zeroizing<[u8; 32]> = Zeroizing::new(opened.as_slice().try_into().map_err(|_| CryptoError::Format)?);
        Ok(Key(*bytes))
    }
}

pub struct LookupKey(Key);

impl LookupKey {
    pub fn new(bytes: Zeroizing<[u8; 32]>) -> LookupKey {
        LookupKey(Key(*bytes))
    }

    pub fn lookup(&self, provider: &str, sub: &str) -> [u8; 32] {
        let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(&self.0.0).expect("HMAC takes any key length");
        mac.update(provider.as_bytes());
        mac.update(&[0]);
        mac.update(sub.as_bytes());
        mac.finalize().into_bytes().into()
    }
}

#[cfg(test)]
mod tests;
