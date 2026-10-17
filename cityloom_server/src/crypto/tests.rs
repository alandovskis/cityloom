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
