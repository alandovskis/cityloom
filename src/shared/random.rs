//! 128 random bits, for ids that must not collide between devices (an area rule made on a phone and one made on a
//! laptop are never the same rule).

/// 32 lowercase hexadecimal digits from the browser's crypto source.
pub fn hex128() -> String {
    bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(target_arch = "wasm32")]
fn bytes() -> [u8; 16] {
    let mut out = [0u8; 16];
    let filled = web_sys::window().and_then(|w| w.crypto().ok()).is_some_and(|c| c.get_random_values_with_u8_array(&mut out).is_ok());
    if !filled {
        // No crypto source (very old browsers): the collision chance of 128 bits of Math.random is still negligible.
        for b in &mut out {
            *b = (js_sys::Math::random() * 256.0) as u8;
        }
    }
    out
}

/// Natively (tests only) the standard library's randomly keyed hasher is the source.
#[cfg(not(target_arch = "wasm32"))]
fn bytes() -> [u8; 16] {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let state = std::collections::hash_map::RandomState::new();
    let mut out = [0u8; 16];
    for half in out.chunks_mut(8) {
        let mut h = state.build_hasher();
        h.write_usize(COUNTER.fetch_add(1, Ordering::Relaxed));
        half.copy_from_slice(&h.finish().to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_32_hex_digits_and_not_the_same_twice() {
        let (a, b) = (hex128(), hex128());
        assert!(a.len() == 32 && a.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)), "{a}");
        assert_ne!(a, b);
    }
}
