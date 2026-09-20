//! `extract_key` — the extract key: BLAKE3 over the canonical box and the
//! `buildId`, and nothing else (BR6.1, PD-1). Pure.

use crate::geo::CanonicalBox;
use crate::manifest::BuildId;

/// The cache and response key for one canonical box on one build (BR6.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExtractKey([u8; 32]);

impl ExtractKey {
    /// BLAKE3 of the box's five-decimal text, a newline, and the buildId
    /// (BR6.1). Equal canonical boxes on the same build share a key; a
    /// different box or a different build never does (BR6.4).
    pub fn derive(canonical_box: &CanonicalBox, build_id: &BuildId) -> ExtractKey {
        let text = format!("{}\n{}", canonical_box.key_text(), build_id.as_str());
        ExtractKey(*blake3::hash(text.as_bytes()).as_bytes())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> ExtractKey {
        ExtractKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The lowercase hex form the response's `x-extract-key` header carries.
    pub fn to_hex(&self) -> String {
        blake3::Hash::from_bytes(self.0).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::CanonicalBox;
    use crate::manifest::BuildId;

    fn build(byte: &str) -> BuildId {
        BuildId::parse(&byte.repeat(32)).unwrap()
    }

    // BR6.1 — the key is the lowercase hex BLAKE3 of the five-decimal box
    // text, a newline, and the buildId.
    #[test]
    fn key_is_blake3_of_box_text_newline_build_id() {
        let b = CanonicalBox::parse("-79.63,43.65,-79.62,43.66").unwrap();
        let key = ExtractKey::derive(&b, &build("ab"));
        let expected = blake3::hash(
            format!("-79.63000,43.65000,-79.62000,43.66000\n{}", "ab".repeat(32)).as_bytes(),
        );
        assert_eq!(key.to_hex(), expected.to_hex().to_string());
        assert_eq!(key.to_hex().len(), 64);
        assert!(
            key.to_hex()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn equal_canonical_boxes_share_a_key() {
        let a = CanonicalBox::parse("-79.63,43.65,-79.62,43.66").unwrap();
        let b = CanonicalBox::parse("-79.630000,43.6500,-79.6200000,43.66000").unwrap();
        assert_eq!(
            ExtractKey::derive(&a, &build("ab")),
            ExtractKey::derive(&b, &build("ab"))
        );
    }

    #[test]
    fn a_different_box_gives_a_different_key() {
        let a = CanonicalBox::parse("-79.63,43.65,-79.62,43.66").unwrap();
        let b = CanonicalBox::parse("-79.63,43.65,-79.62,43.66001").unwrap();
        assert_ne!(
            ExtractKey::derive(&a, &build("ab")),
            ExtractKey::derive(&b, &build("ab"))
        );
    }

    // BR6.4 — the same box on another build has another key.
    #[test]
    fn a_different_build_gives_a_different_key() {
        let a = CanonicalBox::parse("-79.63,43.65,-79.62,43.66").unwrap();
        assert_ne!(
            ExtractKey::derive(&a, &build("ab")),
            ExtractKey::derive(&a, &build("cd"))
        );
    }

    #[test]
    fn key_bytes_and_hex_agree() {
        let a = CanonicalBox::parse("0,0,1,1").unwrap();
        let key = ExtractKey::derive(&a, &build("00"));
        assert_eq!(ExtractKey::from_bytes(*key.as_bytes()), key);
        assert_eq!(
            key.to_hex(),
            blake3::Hash::from_bytes(*key.as_bytes())
                .to_hex()
                .to_string()
        );
    }
}
