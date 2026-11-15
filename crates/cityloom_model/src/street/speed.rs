//! A street's speed limit, and its zones'.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Street, Zone};

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

/// The speed of zone `z`: its own, else the street's. With no zones the street's.
pub fn speed_in(base: Speed, zones: &[Zone], z: usize) -> Speed {
    match zones.get(z) {
        Some(zone) if !zone.speed.is_undecided() => zone.speed,
        _ => base,
    }
}

/// The highest speed any zone has, when one has a speed.
pub fn max_kmh(base: Speed, zones: &[Zone]) -> Option<u16> {
    (0..zones.len().max(1)).filter_map(|z| speed_in(base, zones, z).kmh()).max()
}

impl Street {
    /// The speed of zone `z`, its own or else the street's.
    pub fn zone_speed(&self, z: usize) -> Speed {
        speed_in(self.speed, &self.zones, z)
    }

    /// The limit of zone `z` in kilometres an hour, when it has one.
    pub fn zone_speed_kmh(&self, z: usize) -> Option<u16> {
        self.zone_speed(z).kmh()
    }

    /// The highest limit any zone has: what an area's speed rule judges the street by.
    pub fn max_speed_kmh(&self) -> Option<u16> {
        max_kmh(self.speed, &self.zones)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_speed_that_is_not_a_speed_is_not_read() {
        assert!(serde_json::from_str::<Speed>("70000").is_err());
        assert!(serde_json::from_str::<Speed>("-1").is_err());
        assert!(serde_json::from_str::<Speed>("\"fast\"").is_err());
    }
}
