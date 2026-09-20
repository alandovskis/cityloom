//! The wire/storage shape for a CityLoom design payload.
//!
//! This crate defines the one serialised structure that crosses the
//! device/server, Rust/database, and export boundaries. It is a leaf
//! data-shape crate: no API layer, no repository, no frontend, and no
//! dependency on `street-core` or any other workspace crate (Unit U3).
//!
//! Field naming on the wire is camelCase throughout (Contract 3), except
//! [`MapConfig`]'s own inner fields, which are snake_case by explicit
//! design (`entities.md`).

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The one serialised shape that crosses every boundary: device/server,
/// Rust/database, and export. `payload_version` exists from release one
/// even though nothing migrates yet (BR1.1).
///
/// Construct this type directly only when you already know the bytes are
/// version 1 (e.g. building a payload to write). To read untrusted bytes,
/// use [`DesignPayload::from_json`], which enforces the fail-closed
/// version check (SD-1) before ever returning a value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignPayload {
    pub payload_version: u32,
    pub design: Design,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edits: Option<Vec<LaneEdit>>,
    pub corrections: Vec<Correction>,
}

/// The version this release writes and the only version
/// [`DesignPayload::from_json`] accepts (BR1.1).
pub const CURRENT_PAYLOAD_VERSION: u32 = 1;

/// The design sub-object required by `DesignPayload.design`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Design {
    pub design_id: String,
    pub name: String,
    pub streets: Vec<StreetKey>,
    pub created_at: String,
    pub updated_at: String,
}

/// The overlay key — OSM way id plus the direction-normalised pair of
/// bounding OSM node ids — never osm2streets' positional indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreetKey {
    pub osm_way_id: u64,
    pub bounding_node_ids: [u64; 2],
}

/// No physical quantity is a bare number once it has crossed the adapter
/// boundary. Metres are the only unit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimension {
    pub metres: f64,
    pub provenance: Provenance,
}

/// Where a [`Dimension`] value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    Mapped,
    Inferred,
    UserSet,
}

/// A proposed change to the baseline — change an existing lane's
/// attribute, add a lane that exists in no import, or remove one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneEdit {
    pub edit_id: String,
    pub street: StreetKey,
    /// Derived from lane type, direction and ordinal from the kerb;
    /// absent on an addition, which carries an `anchor` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_key: Option<String>,
    /// Position relative to a keyed baseline lane, for a lane that
    /// exists in no import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<JsonValue>,
    pub kind: LaneEditKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<LaneEditAttribute>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Dimension>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaneEditKind {
    Change,
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaneEditAttribute {
    LaneType,
    Width,
    Direction,
}

/// States that the import itself was wrong, so it is permanent. Its
/// provenance is always `UserSet` and never returns to `Mapped` —
/// enforced structurally: this type carries NO `provenance` field at
/// all (SD-2, BR4.1). Carries a fingerprint a design edit has no use
/// for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    pub correction_id: String,
    pub street: StreetKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Dimension>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub state: CorrectionState,
    pub import_fingerprint: ImportFingerprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionState {
    Applied,
    ReappliedAfterChange,
    Unresolved,
}

/// What was imported at the moment a [`Correction`] was made. Every
/// field is required, none optional (BR5.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFingerprint {
    pub way_tags: JsonValue,
    pub map_config: MapConfig,
    pub osm2streets_revision: String,
}

/// `mapConfig`'s inner fields are explicitly snake_case (`entities.md`),
/// unlike every other field in this crate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MapConfig {
    pub country_code: String,
    pub driving_side: String,
    pub inferred_sidewalks: bool,
    pub inferred_kerbs: bool,
}

/// A minimal, self-contained JSON value, hand-rolled so this crate needs
/// no runtime dependency beyond `serde` itself (`tech-stack-decisions.md`
/// keeps `serde_json` a dev-only dependency, used only to exercise these
/// types in tests). Used for the free-form sub-objects `entities.md`
/// types as `object` without fixing their internal shape here: `anchor`
/// and `wayTags`.
///
/// Object keys are stored in a [`BTreeMap`], so serialization always
/// emits keys in sorted order regardless of insertion order — this keeps
/// byte-for-byte round-trip comparisons deterministic.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum JsonValue {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            JsonValue::Null => serializer.serialize_unit(),
            JsonValue::Bool(value) => serializer.serialize_bool(*value),
            JsonValue::Number(value) => serializer.serialize_f64(*value),
            JsonValue::String(value) => serializer.serialize_str(value),
            JsonValue::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            JsonValue::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JsonValueVisitor;

        impl<'de> Visitor<'de> for JsonValueVisitor {
            type Value = JsonValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value (null, bool, number, string, array or object)")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(JsonValue::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(JsonValue::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(JsonValue::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(JsonValue::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValue::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(JsonValue::String(value))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(JsonValue::Null)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(JsonValue::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(JsonValue::Array(items))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = BTreeMap::new();
                while let Some((key, value)) = map.next_entry()? {
                    entries.insert(key, value);
                }
                Ok(JsonValue::Object(entries))
            }
        }

        deserializer.deserialize_any(JsonValueVisitor)
    }
}

/// A closed set of failure reasons for reading payload bytes. Never lets
/// an underlying `serde_json`-shaped error type reach a caller (SD-1).
#[derive(Debug)]
pub enum PayloadError {
    /// `payloadVersion` was present and well-formed, but is not the
    /// version this crate accepts (BR1.1). The `DesignPayload` is never
    /// constructed from the mismatched-version bytes.
    UnsupportedPayloadVersion(u32),
    /// The bytes did not parse as JSON, or did not match the expected
    /// shape at all (including a missing/malformed `payloadVersion`).
    Malformed(String),
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadError::UnsupportedPayloadVersion(version) => {
                write!(f, "unsupported payload version: {version}")
            }
            PayloadError::Malformed(reason) => write!(f, "malformed design payload: {reason}"),
        }
    }
}

impl std::error::Error for PayloadError {}

/// Only the field this fail-closed check needs to read before deciding
/// whether the rest of the bytes are worth trusting.
#[derive(Deserialize)]
struct PayloadVersionProbe {
    #[serde(rename = "payloadVersion")]
    payload_version: u32,
}

impl DesignPayload {
    /// Deserialize payload bytes, enforcing the fail-closed version check
    /// (SD-1, BR1.1) before ever returning a value: the version is read
    /// and validated first, and a `DesignPayload` is constructed only
    /// when it equals [`CURRENT_PAYLOAD_VERSION`]. An unrecognised
    /// version is a stated, handled outcome
    /// (`PayloadError::UnsupportedPayloadVersion`), never a crash or a
    /// silent misread into the current shape.
    ///
    /// Parses the bytes itself via this crate's own minimal JSON reader
    /// (see [`json_reader`]) rather than `serde_json`, which stays a
    /// dev-only dependency (`tech-stack-decisions.md`): the actual wire
    /// encoding this method reads is real JSON text, but the crate that
    /// owns the encoding decision for storage/transport is
    /// `u10-design-storage`, not this one.
    pub fn from_json(bytes: &str) -> Result<DesignPayload, PayloadError> {
        let probe: PayloadVersionProbe = json_reader::from_str(bytes)
            .map_err(|error| PayloadError::Malformed(error.to_string()))?;

        if probe.payload_version != CURRENT_PAYLOAD_VERSION {
            return Err(PayloadError::UnsupportedPayloadVersion(
                probe.payload_version,
            ));
        }

        json_reader::from_str(bytes).map_err(|error| PayloadError::Malformed(error.to_string()))
    }
}

/// A minimal JSON text reader implementing `serde::Deserializer`, used
/// only by [`DesignPayload::from_json`]. Exists so this crate can read
/// real JSON bytes without taking `serde_json` as a runtime dependency —
/// `serde` (already a dependency for the derives) is all this needs.
///
/// Supports exactly the JSON subset this crate's types require: object,
/// array, string (with standard escapes, including `\uXXXX` and
/// surrogate pairs), number (integer or float), `true`/`false`/`null`,
/// and unit-variant enums encoded as a bare JSON string.
mod json_reader {
    use std::fmt;

    use serde::de::{
        self, DeserializeSeed, Error as _, IntoDeserializer, MapAccess, SeqAccess, Visitor,
    };
    use serde::{Deserialize, Deserializer};

    /// A JSON parsing failure. Its `Display` text is the only detail
    /// `PayloadError::Malformed` carries forward past this boundary.
    #[derive(Debug)]
    pub struct Error(String);

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for Error {}

    impl de::Error for Error {
        fn custom<T>(msg: T) -> Self
        where
            T: fmt::Display,
        {
            Error(msg.to_string())
        }
    }

    /// Parse `input` as a complete JSON document into `T`, rejecting any
    /// trailing bytes after the value.
    pub fn from_str<'de, T>(input: &'de str) -> Result<T, Error>
    where
        T: Deserialize<'de>,
    {
        let mut reader = Reader::new(input);
        let value = T::deserialize(&mut reader)?;
        reader.skip_whitespace();
        if reader.pos != reader.input.len() {
            return Err(Error::custom("trailing characters after JSON value"));
        }
        Ok(value)
    }

    enum Number {
        Unsigned(u64),
        Signed(i64),
        Float(f64),
    }

    struct Reader<'de> {
        input: &'de str,
        pos: usize,
    }

    impl<'de> Reader<'de> {
        fn new(input: &'de str) -> Self {
            Reader { input, pos: 0 }
        }

        fn rest(&self) -> &'de str {
            &self.input[self.pos..]
        }

        fn skip_whitespace(&mut self) {
            while let Some(ch) = self.rest().chars().next() {
                if ch.is_ascii_whitespace() {
                    self.pos += ch.len_utf8();
                } else {
                    break;
                }
            }
        }

        fn peek(&self) -> Result<char, Error> {
            self.rest()
                .chars()
                .next()
                .ok_or_else(|| Error::custom("unexpected end of JSON input"))
        }

        fn bump(&mut self) -> Result<char, Error> {
            let ch = self.peek()?;
            self.pos += ch.len_utf8();
            Ok(ch)
        }

        fn expect(&mut self, expected: char) -> Result<(), Error> {
            let found = self.bump()?;
            if found == expected {
                Ok(())
            } else {
                Err(Error::custom(format!(
                    "expected '{expected}', found '{found}'"
                )))
            }
        }

        fn expect_literal(&mut self, literal: &str) -> Result<(), Error> {
            if self.rest().starts_with(literal) {
                self.pos += literal.len();
                Ok(())
            } else {
                Err(Error::custom(format!("expected literal '{literal}'")))
            }
        }

        fn parse_string(&mut self) -> Result<String, Error> {
            self.expect('"')?;
            let mut result = String::new();
            loop {
                let ch = self.bump()?;
                match ch {
                    '"' => return Ok(result),
                    '\\' => self.parse_escape(&mut result)?,
                    other => result.push(other),
                }
            }
        }

        fn parse_escape(&mut self, result: &mut String) -> Result<(), Error> {
            let escaped = self.bump()?;
            match escaped {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                '/' => result.push('/'),
                'b' => result.push('\u{0008}'),
                'f' => result.push('\u{000C}'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                'u' => {
                    let unit = self.parse_hex4()?;
                    let decoded = if (0xD800..=0xDBFF).contains(&unit) {
                        self.expect('\\')?;
                        self.expect('u')?;
                        let low = self.parse_hex4()?;
                        if !(0xDC00..=0xDFFF).contains(&low) {
                            return Err(Error::custom("invalid surrogate pair in JSON string"));
                        }
                        let combined = 0x10000
                            + ((u32::from(unit) - 0xD800) << 10)
                            + (u32::from(low) - 0xDC00);
                        char::from_u32(combined)
                    } else {
                        char::from_u32(u32::from(unit))
                    };
                    result.push(
                        decoded.ok_or_else(|| {
                            Error::custom("invalid unicode escape in JSON string")
                        })?,
                    );
                }
                other => {
                    return Err(Error::custom(format!(
                        "invalid escape sequence '\\{other}'"
                    )));
                }
            }
            Ok(())
        }

        fn parse_hex4(&mut self) -> Result<u16, Error> {
            let mut value: u16 = 0;
            for _ in 0..4 {
                let ch = self.bump()?;
                let digit = ch
                    .to_digit(16)
                    .ok_or_else(|| Error::custom("invalid unicode escape in JSON string"))?;
                value = value * 16 + digit as u16;
            }
            Ok(value)
        }

        fn parse_number(&mut self) -> Result<Number, Error> {
            let start = self.pos;
            let mut is_float = false;
            if self.peek()? == '-' {
                self.bump()?;
            }
            self.consume_digits();
            if self.peek().is_ok_and(|ch| ch == '.') {
                is_float = true;
                self.bump()?;
                self.consume_digits();
            }
            if self.peek().is_ok_and(|ch| ch == 'e' || ch == 'E') {
                is_float = true;
                self.bump()?;
                if self.peek().is_ok_and(|ch| ch == '+' || ch == '-') {
                    self.bump()?;
                }
                self.consume_digits();
            }

            let literal = &self.input[start..self.pos];
            if literal.is_empty() || literal == "-" {
                return Err(Error::custom("invalid number literal in JSON input"));
            }
            if is_float {
                return literal
                    .parse::<f64>()
                    .map(Number::Float)
                    .map_err(|_| Error::custom(format!("invalid number literal '{literal}'")));
            }
            if let Ok(value) = literal.parse::<u64>() {
                return Ok(Number::Unsigned(value));
            }
            if let Ok(value) = literal.parse::<i64>() {
                return Ok(Number::Signed(value));
            }
            literal
                .parse::<f64>()
                .map(Number::Float)
                .map_err(|_| Error::custom(format!("invalid number literal '{literal}'")))
        }

        fn consume_digits(&mut self) {
            while self.peek().is_ok_and(|ch| ch.is_ascii_digit()) {
                let _ = self.bump();
            }
        }
    }

    impl<'de> Deserializer<'de> for &mut Reader<'de> {
        type Error = Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            self.skip_whitespace();
            match self.peek()? {
                'n' => {
                    self.expect_literal("null")?;
                    visitor.visit_unit()
                }
                't' => {
                    self.expect_literal("true")?;
                    visitor.visit_bool(true)
                }
                'f' => {
                    self.expect_literal("false")?;
                    visitor.visit_bool(false)
                }
                '"' => {
                    let value = self.parse_string()?;
                    visitor.visit_string(value)
                }
                '[' => {
                    self.bump()?;
                    let value = visitor.visit_seq(CommaSeparated::new(self))?;
                    self.skip_whitespace();
                    self.expect(']')?;
                    Ok(value)
                }
                '{' => {
                    self.bump()?;
                    let value = visitor.visit_map(CommaSeparated::new(self))?;
                    self.skip_whitespace();
                    self.expect('}')?;
                    Ok(value)
                }
                '-' | '0'..='9' => match self.parse_number()? {
                    Number::Unsigned(value) => visitor.visit_u64(value),
                    Number::Signed(value) => visitor.visit_i64(value),
                    Number::Float(value) => visitor.visit_f64(value),
                },
                other => Err(Error::custom(format!(
                    "unexpected character '{other}' while parsing a JSON value"
                ))),
            }
        }

        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            self.skip_whitespace();
            if self.rest().starts_with("null") {
                self.pos += "null".len();
                visitor.visit_none()
            } else {
                visitor.visit_some(self)
            }
        }

        fn deserialize_enum<V>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            self.skip_whitespace();
            let variant = self.parse_string()?;
            visitor.visit_enum(IntoDeserializer::<Error>::into_deserializer(variant))
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf unit unit_struct newtype_struct seq tuple
            tuple_struct map struct identifier ignored_any
        }
    }

    struct CommaSeparated<'a, 'de> {
        reader: &'a mut Reader<'de>,
        first: bool,
    }

    impl<'a, 'de> CommaSeparated<'a, 'de> {
        fn new(reader: &'a mut Reader<'de>) -> Self {
            CommaSeparated {
                reader,
                first: true,
            }
        }
    }

    impl<'de> SeqAccess<'de> for CommaSeparated<'_, 'de> {
        type Error = Error;

        fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
        where
            T: DeserializeSeed<'de>,
        {
            self.reader.skip_whitespace();
            if self.reader.peek()? == ']' {
                return Ok(None);
            }
            if !self.first {
                self.reader.expect(',')?;
                self.reader.skip_whitespace();
            }
            self.first = false;
            seed.deserialize(&mut *self.reader).map(Some)
        }
    }

    impl<'de> MapAccess<'de> for CommaSeparated<'_, 'de> {
        type Error = Error;

        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
        where
            K: DeserializeSeed<'de>,
        {
            self.reader.skip_whitespace();
            if self.reader.peek()? == '}' {
                return Ok(None);
            }
            if !self.first {
                self.reader.expect(',')?;
                self.reader.skip_whitespace();
            }
            self.first = false;
            seed.deserialize(&mut *self.reader).map(Some)
        }

        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
        where
            V: DeserializeSeed<'de>,
        {
            self.reader.skip_whitespace();
            self.reader.expect(':')?;
            self.reader.skip_whitespace();
            seed.deserialize(&mut *self.reader)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CURRENT_PAYLOAD_VERSION, Correction, CorrectionState, Design, DesignPayload, Dimension,
        ImportFingerprint, JsonValue, LaneEdit, LaneEditAttribute, LaneEditKind, MapConfig,
        PayloadError, Provenance, StreetKey,
    };
    use std::collections::BTreeMap;

    fn sample_anchor() -> JsonValue {
        let mut entries = BTreeMap::new();
        entries.insert(
            "relativeTo".to_string(),
            JsonValue::String("driving-forward-0".to_string()),
        );
        entries.insert("side".to_string(), JsonValue::String("left".to_string()));
        JsonValue::Object(entries)
    }

    fn sample_way_tags(highway: &str, lanes: Option<&str>) -> JsonValue {
        let mut entries = BTreeMap::new();
        entries.insert(
            "highway".to_string(),
            JsonValue::String(highway.to_string()),
        );
        if let Some(lanes) = lanes {
            entries.insert("lanes".to_string(), JsonValue::String(lanes.to_string()));
        }
        JsonValue::Object(entries)
    }

    fn sample_map_config() -> MapConfig {
        MapConfig {
            country_code: "GB".to_string(),
            driving_side: "left".to_string(),
            inferred_sidewalks: true,
            inferred_kerbs: true,
        }
    }

    #[test]
    fn full_design_payload_round_trips_byte_for_byte() {
        let payload = DesignPayload {
            payload_version: CURRENT_PAYLOAD_VERSION,
            design: Design {
                design_id: "design-1".to_string(),
                name: "Main Street redesign".to_string(),
                streets: vec![],
                created_at: "2026-09-18T00:00:00Z".to_string(),
                updated_at: "2026-09-18T00:00:00Z".to_string(),
            },
            edits: Some(vec![
                LaneEdit {
                    edit_id: "edit-1".to_string(),
                    street: StreetKey {
                        osm_way_id: 42,
                        bounding_node_ids: [100, 200],
                    },
                    lane_key: Some("driving-forward-0".to_string()),
                    anchor: None,
                    kind: LaneEditKind::Change,
                    attribute: Some(LaneEditAttribute::Width),
                    width: Some(Dimension {
                        metres: 3.5,
                        provenance: Provenance::UserSet,
                    }),
                    value: None,
                },
                LaneEdit {
                    edit_id: "edit-2".to_string(),
                    street: StreetKey {
                        osm_way_id: 42,
                        bounding_node_ids: [100, 200],
                    },
                    lane_key: None,
                    anchor: Some(sample_anchor()),
                    kind: LaneEditKind::Add,
                    attribute: None,
                    width: None,
                    value: None,
                },
            ]),
            corrections: vec![Correction {
                correction_id: "correction-1".to_string(),
                street: StreetKey {
                    osm_way_id: 42,
                    bounding_node_ids: [100, 200],
                },
                lane_key: Some("driving-forward-0".to_string()),
                attribute: Some("laneType".to_string()),
                width: None,
                value: Some("cycleway".to_string()),
                state: CorrectionState::Applied,
                import_fingerprint: ImportFingerprint {
                    way_tags: sample_way_tags("residential", None),
                    map_config: sample_map_config(),
                    osm2streets_revision: "fc119c47dac567d030c6ce7c24a48896f58ed906".to_string(),
                },
            }],
        };

        let json_string = serde_json::to_string(&payload).expect("serialize DesignPayload");
        let round_tripped: DesignPayload =
            serde_json::from_str(&json_string).expect("deserialize DesignPayload");
        let round_tripped_json_string =
            serde_json::to_string(&round_tripped).expect("re-serialize DesignPayload");

        assert_eq!(json_string, round_tripped_json_string);
        assert_eq!(payload, round_tripped);
    }

    #[test]
    fn inferred_dimension_round_trips_with_provenance_intact() {
        let dimension = Dimension {
            metres: 2.75,
            provenance: Provenance::Inferred,
        };

        let json_string = serde_json::to_string(&dimension).expect("serialize Dimension");
        let round_tripped: Dimension =
            serde_json::from_str(&json_string).expect("deserialize Dimension");

        assert_eq!(round_tripped.provenance, Provenance::Inferred);
        assert_eq!(round_tripped.metres, 2.75);
    }

    #[test]
    fn mapped_and_user_set_dimensions_round_trip_distinctly() {
        let mapped = Dimension {
            metres: 3.0,
            provenance: Provenance::Mapped,
        };
        let user_set = Dimension {
            metres: 3.2,
            provenance: Provenance::UserSet,
        };

        let mapped_json = serde_json::to_string(&mapped).expect("serialize mapped Dimension");
        let user_set_json = serde_json::to_string(&user_set).expect("serialize user-set Dimension");

        let mapped_round_tripped: Dimension =
            serde_json::from_str(&mapped_json).expect("deserialize mapped Dimension");
        let user_set_round_tripped: Dimension =
            serde_json::from_str(&user_set_json).expect("deserialize user-set Dimension");

        assert_eq!(mapped_round_tripped.provenance, Provenance::Mapped);
        assert_eq!(user_set_round_tripped.provenance, Provenance::UserSet);
        assert_ne!(mapped_json, user_set_json);
    }

    #[test]
    fn remove_kind_lane_edit_round_trips() {
        let edit = LaneEdit {
            edit_id: "edit-3".to_string(),
            street: StreetKey {
                osm_way_id: 7,
                bounding_node_ids: [10, 20],
            },
            lane_key: Some("cycleway-forward-0".to_string()),
            anchor: None,
            kind: LaneEditKind::Remove,
            attribute: None,
            width: None,
            value: None,
        };

        let json_string = serde_json::to_string(&edit).expect("serialize LaneEdit");
        let round_tripped: LaneEdit =
            serde_json::from_str(&json_string).expect("deserialize LaneEdit");

        assert_eq!(round_tripped.kind, LaneEditKind::Remove);
        assert_eq!(round_tripped.attribute, None);
        assert_eq!(round_tripped.width, None);
        assert_eq!(round_tripped.value, None);
        assert_eq!(round_tripped, edit);
    }

    #[test]
    fn correction_serializes_with_no_provenance_key() {
        let correction = Correction {
            correction_id: "correction-2".to_string(),
            street: StreetKey {
                osm_way_id: 7,
                bounding_node_ids: [10, 20],
            },
            lane_key: Some("cycleway-forward-0".to_string()),
            attribute: Some("width".to_string()),
            width: Some(Dimension {
                metres: 1.8,
                provenance: Provenance::UserSet,
            }),
            value: None,
            state: CorrectionState::Unresolved,
            import_fingerprint: ImportFingerprint {
                way_tags: sample_way_tags("cycleway", None),
                map_config: MapConfig {
                    country_code: "GB".to_string(),
                    driving_side: "left".to_string(),
                    inferred_sidewalks: false,
                    inferred_kerbs: false,
                },
                osm2streets_revision: "fc119c47dac567d030c6ce7c24a48896f58ed906".to_string(),
            },
        };

        let json_string = serde_json::to_string(&correction).expect("serialize Correction");
        let value: serde_json::Value =
            serde_json::from_str(&json_string).expect("parse serialized Correction as JSON");
        let object = value
            .as_object()
            .expect("Correction serializes as an object");

        assert!(
            !object.contains_key("provenance"),
            "Correction must never carry a provenance key on the wire: {object:?}"
        );
    }

    #[test]
    fn import_fingerprint_round_trips_completely() {
        let fingerprint = ImportFingerprint {
            way_tags: sample_way_tags("residential", Some("2")),
            map_config: MapConfig {
                country_code: "US".to_string(),
                driving_side: "right".to_string(),
                inferred_sidewalks: true,
                inferred_kerbs: false,
            },
            osm2streets_revision: "fc119c47dac567d030c6ce7c24a48896f58ed906".to_string(),
        };

        let json_string = serde_json::to_string(&fingerprint).expect("serialize ImportFingerprint");
        let round_tripped: ImportFingerprint =
            serde_json::from_str(&json_string).expect("deserialize ImportFingerprint");

        assert_eq!(round_tripped, fingerprint);
        assert!(json_string.contains("\"country_code\":\"US\""));
        assert!(json_string.contains("\"driving_side\":\"right\""));
        assert!(json_string.contains("\"inferred_sidewalks\":true"));
        assert!(json_string.contains("\"inferred_kerbs\":false"));
    }

    #[test]
    fn wire_field_names_are_exact_camel_case() {
        let street_key = StreetKey {
            osm_way_id: 99,
            bounding_node_ids: [1, 2],
        };

        let json_string = serde_json::to_string(&street_key).expect("serialize StreetKey");

        assert!(
            json_string.contains("\"osmWayId\":99"),
            "expected camelCase osmWayId in {json_string}"
        );
        assert!(
            json_string.contains("\"boundingNodeIds\":[1,2]"),
            "expected camelCase boundingNodeIds in {json_string}"
        );

        let payload = DesignPayload {
            payload_version: CURRENT_PAYLOAD_VERSION,
            design: Design {
                design_id: "design-2".to_string(),
                name: "Field naming check".to_string(),
                streets: vec![],
                created_at: "2026-09-18T00:00:00Z".to_string(),
                updated_at: "2026-09-18T00:00:00Z".to_string(),
            },
            edits: None,
            corrections: vec![],
        };
        let payload_json_string = serde_json::to_string(&payload).expect("serialize DesignPayload");
        assert!(payload_json_string.contains("\"payloadVersion\":1"));
    }

    #[test]
    fn version_one_parses_to_ok() {
        let bytes = r#"{
            "payloadVersion": 1,
            "design": {
                "designId": "design-1",
                "name": "Main Street redesign",
                "streets": [],
                "createdAt": "2026-09-18T00:00:00Z",
                "updatedAt": "2026-09-18T00:00:00Z"
            },
            "corrections": []
        }"#;

        let payload = DesignPayload::from_json(bytes).expect("version 1 payload parses");

        assert_eq!(payload.payload_version, 1);
        assert_eq!(payload.design.design_id, "design-1");
    }

    #[test]
    fn unsupported_version_is_rejected_without_constructing_a_payload() {
        let bytes = r#"{
            "payloadVersion": 2,
            "design": {
                "designId": "design-1",
                "name": "Main Street redesign",
                "streets": [],
                "createdAt": "2026-09-18T00:00:00Z",
                "updatedAt": "2026-09-18T00:00:00Z"
            },
            "corrections": []
        }"#;

        let result = DesignPayload::from_json(bytes);

        match result {
            Err(PayloadError::UnsupportedPayloadVersion(2)) => {}
            other => panic!("expected Err(UnsupportedPayloadVersion(2)), got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_returns_a_distinct_error_variant_never_a_panic() {
        let bytes = "{ this is not valid JSON";

        let result = DesignPayload::from_json(bytes);

        match result {
            Err(PayloadError::Malformed(_)) => {}
            other => panic!("expected Err(Malformed(_)), got {other:?}"),
        }
    }
}
