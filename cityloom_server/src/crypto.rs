//! Keys, sealing and the identity lookup hash. Nothing here is logged or put in an error.
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

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
        let mut opened = open(self.get(key_id)?, &Self::wrap_aad(user), wrapped)?;
        let bytes: [u8; 32] = opened.as_slice().try_into().map_err(|_| CryptoError::Format)?;
        opened.zeroize();
        Ok(Key(bytes))
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
mod tests {
    use super::*;

    fn key(n: u8) -> Key {
        Key::from_bytes([n; 32])
    }

    #[test]
    fn a_sealed_value_opens_to_what_was_sealed() {
        let a = aad("users", "display_name", "u1");
        let sealed = seal(&key(1), &a, b"Ada Lovelace");
        assert_eq!(&*open(&key(1), &a, &sealed).unwrap(), b"Ada Lovelace");
        assert_eq!(&*open(&key(1), &a, &seal(&key(1), &a, b"")).unwrap(), b"");
    }

    #[test]
    fn the_format_is_version_nonce_ciphertext_tag() {
        let sealed = seal(&key(1), "t.c.i", b"abc");
        assert_eq!(sealed[0], SEAL_VERSION);
        assert_eq!(sealed.len(), 1 + 12 + 3 + 16);
    }

    #[test]
    fn two_seals_of_one_value_differ() {
        let a = aad("users", "display_name", "u1");
        assert_ne!(seal(&key(1), &a, b"same"), seal(&key(1), &a, b"same"));
    }

    #[test]
    fn a_sealed_value_is_refused_under_another_row_column_or_data_key() {
        let sealed = seal(&key(1), &aad("users", "display_name", "u1"), b"x");
        assert_eq!(open(&key(1), &aad("users", "display_name", "u2"), &sealed).unwrap_err(), CryptoError::Open);
        assert_eq!(open(&key(1), &aad("users", "store_on_server", "u1"), &sealed).unwrap_err(), CryptoError::Open);
        assert_eq!(open(&key(1), &aad("identities", "display_name", "u1"), &sealed).unwrap_err(), CryptoError::Open);
        assert_eq!(open(&key(2), &aad("users", "display_name", "u1"), &sealed).unwrap_err(), CryptoError::Open);
    }

    #[test]
    fn a_sealed_value_with_any_byte_changed_is_refused() {
        let a = aad("users", "display_name", "u1");
        let sealed = seal(&key(1), &a, b"hello there");
        for i in 0..sealed.len() {
            let mut bad = sealed.clone();
            bad[i] ^= 1;
            assert!(open(&key(1), &a, &bad).is_err(), "byte {i}");
        }
        assert_eq!(open(&key(1), &a, &sealed[..10]).unwrap_err(), CryptoError::Format);
        assert_eq!(open(&key(1), &a, &[]).unwrap_err(), CryptoError::Format);
    }

    #[test]
    fn an_unknown_version_is_a_format_error() {
        let a = "t.c.i";
        let mut sealed = seal(&key(1), a, b"x");
        sealed[0] = 2;
        assert_eq!(open(&key(1), a, &sealed).unwrap_err(), CryptoError::Format);
    }

    #[test]
    fn a_data_key_is_wrapped_under_the_current_master_key_and_bound_to_its_user() {
        let masters = MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32])), (2, zeroize::Zeroizing::new([2; 32]))], 2).unwrap();
        let user = uuid::Uuid::new_v4();
        let data = Key::random();
        let wrapped = masters.wrap(user, &data);
        assert_eq!(masters.current_id(), 2);
        let back = masters.unwrap(2, user, &wrapped).unwrap();
        assert_eq!(back.0, data.0);
        assert!(masters.unwrap(2, uuid::Uuid::new_v4(), &wrapped).is_err(), "another user");
        assert!(masters.unwrap(1, user, &wrapped).is_err(), "another master key");
        assert_eq!(masters.unwrap(9, user, &wrapped).unwrap_err(), CryptoError::UnknownKeyId(9));
        let other = MasterKeys::new(&[(2, zeroize::Zeroizing::new([3; 32]))], 2).unwrap();
        assert!(other.unwrap(2, user, &wrapped).is_err(), "a different master key under the same id");
    }

    #[test]
    fn master_keys_need_the_current_one_listed_and_no_id_twice() {
        assert!(MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32]))], 2).is_err());
        assert!(MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32])), (1, zeroize::Zeroizing::new([2; 32]))], 1).is_err());
    }

    #[test]
    fn the_lookup_hash_depends_on_provider_sub_and_key_and_separates_them() {
        let k = LookupKey::new(zeroize::Zeroizing::new([5; 32]));
        assert_eq!(k.lookup("google", "123"), k.lookup("google", "123"));
        assert_ne!(k.lookup("google", "123"), k.lookup("google", "124"));
        assert_ne!(k.lookup("google", "123"), k.lookup("apple", "123"));
        assert_ne!(k.lookup("ab", "c"), k.lookup("a", "bc"), "the 0x00 keeps provider and sub apart");
        assert_ne!(k.lookup("google", "123"), LookupKey::new(zeroize::Zeroizing::new([6; 32])).lookup("google", "123"));
    }

    #[test]
    fn debug_never_shows_key_bytes() {
        assert_eq!(format!("{:?}", key(7)), "Key(..)");
    }
}
