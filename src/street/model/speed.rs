//! A street's speed limit, as the city keeps it.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Street;

/// The speed limit of a street, in kilometres an hour. A record written before streets had one has not
/// decided (`Undecided`), and takes the road's when it is applied; `NotSet` is a decision: the road has none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Speed {
    #[default]
    Undecided,
    NotSet,
    Kmh(u16),
}

impl Speed {
    /// What a road says: a limit, or none.
    pub fn from_kmh(kmh: Option<u16>) -> Speed {
        kmh.map_or(Speed::NotSet, Speed::Kmh)
    }

    pub fn is_undecided(&self) -> bool {
        *self == Speed::Undecided
    }

    /// The limit in kilometres an hour, when there is one.
    pub fn kmh(&self) -> Option<u16> {
        match self {
            Speed::Kmh(n) => Some(*n),
            Speed::Undecided | Speed::NotSet => None,
        }
    }
}

impl Street {
    /// The speed limit in kilometres an hour, when the street has one.
    pub fn speed_kmh(&self) -> Option<u16> {
        self.speed.kmh()
    }
}

/// Written as a number, or `null` for `NotSet`; the field is left out for `Undecided` (see `Street::speed`).
impl Serialize for Speed {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Speed::Kmh(n) => s.serialize_u16(*n),
            Speed::Undecided | Speed::NotSet => s.serialize_none(),
        }
    }
}

struct SpeedVisitor;

impl Visitor<'_> for SpeedVisitor {
    type Value = Speed;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a speed in kilometres an hour, or null")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Speed, E> {
        Ok(Speed::NotSet)
    }

    fn visit_none<E: de::Error>(self) -> Result<Speed, E> {
        Ok(Speed::NotSet)
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Speed, E> {
        u16::try_from(n).map(Speed::Kmh).map_err(|_| E::custom("speed out of range"))
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Speed, E> {
        u16::try_from(n).map(Speed::Kmh).map_err(|_| E::custom("speed out of range"))
    }
}

impl<'de> Deserialize<'de> for Speed {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Speed, D::Error> {
        d.deserialize_any(SpeedVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;

    fn street_with(speed: Speed) -> Street {
        let mut street = Street::sample(0, Side::Right);
        street.speed = speed;
        street
    }

    #[test]
    fn speed_round_trips_as_absent_null_or_number() {
        for (speed, written) in [(Speed::Undecided, None), (Speed::NotSet, Some("\"speed_kmh\":null")), (Speed::Kmh(30), Some("\"speed_kmh\":30"))] {
            let json = serde_json::to_string(&street_with(speed)).unwrap();
            match written {
                Some(text) => assert!(json.contains(text), "{speed:?}: {json}"),
                None => assert!(!json.contains("speed_kmh"), "{speed:?}: {json}"),
            }
            let back: Street = serde_json::from_str(&json).unwrap();
            assert_eq!(back.speed, speed);
            assert_eq!(back, street_with(speed));
        }
    }

    #[test]
    fn a_speed_that_is_not_a_speed_is_not_read() {
        assert!(serde_json::from_str::<Speed>("70000").is_err());
        assert!(serde_json::from_str::<Speed>("-1").is_err());
        assert!(serde_json::from_str::<Speed>("\"fast\"").is_err());
    }
}
