//! `geo` — the requested area: `bbox` parsing and validation (BR3.1) and the
//! canonical integer box (BR3.2, PD-1). Pure: no I/O, no logging.
//!
//! The raw `bbox` text is parsed as exactly four plain decimals (no
//! exponent, no sign but `-`, no whitespace, no negative zero) in WGS84
//! order, validated for range and strict `min < max`, and converted **once**
//! to integer units of 1e-5 degree — minimums floored, maximums ceiled, so
//! canonicalisation never shrinks a box (`performance-requirements.md`,
//! "Canonicalisation precision"). No floating-point value exists on the
//! request path (PD-1, SD-5).

use std::cmp::Ordering;
use std::fmt;

use thiserror::Error;

/// Integer units per degree: the canonical precision is five decimals.
pub const UNITS_PER_DEGREE: i64 = 100_000;
/// Nanodegrees per unit: the PBF format stores coordinates in nanodegrees.
pub const NANO_PER_UNIT: i64 = 10_000;

const MAX_LON: i64 = 180 * UNITS_PER_DEGREE;
const MAX_LAT: i64 = 90 * UNITS_PER_DEGREE;
const MAX_INTEGER_DIGITS: usize = 18;

/// The `bbox` could not be read (BR3.1, `400 invalid_area`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("the area could not be read")]
pub struct InvalidArea;

/// A coordinate in integer units of 1e-5 degree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Units(pub i64);

impl Units {
    /// Floor division, toward negative infinity, so cell edges are exact.
    pub fn floor_div(self, divisor: i64) -> i64 {
        self.0.div_euclid(divisor)
    }
}

/// A validated, canonical bounding box: `min < max` on both axes, in range,
/// in integer units (BR3.2). The only form anything after validation sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CanonicalBox {
    pub min_lon: Units,
    pub min_lat: Units,
    pub max_lon: Units,
    pub max_lat: Units,
}

impl CanonicalBox {
    /// Parse, validate (BR3.1) and canonicalise (BR3.2) a `bbox` value.
    pub fn parse(bbox: &str) -> Result<CanonicalBox, InvalidArea> {
        let mut parts = bbox.split(',');
        let mut next = || parts.next().ok_or(InvalidArea).and_then(Decimal::parse);
        let (min_lon, min_lat, max_lon, max_lat) = (next()?, next()?, next()?, next()?);
        if parts.next().is_some() {
            return Err(InvalidArea);
        }
        if min_lon.cmp(&max_lon) != Ordering::Less || min_lat.cmp(&max_lat) != Ordering::Less {
            return Err(InvalidArea);
        }
        let boxed = CanonicalBox {
            min_lon: min_lon.to_units(Rounding::Floor)?,
            min_lat: min_lat.to_units(Rounding::Floor)?,
            max_lon: max_lon.to_units(Rounding::Ceil)?,
            max_lat: max_lat.to_units(Rounding::Ceil)?,
        };
        // Range on the raw values: floor >= -max and ceil <= max together
        // mean the exact decimal lies inside the closed range.
        for (value, limit) in [
            (&min_lon, MAX_LON),
            (&max_lon, MAX_LON),
            (&min_lat, MAX_LAT),
            (&max_lat, MAX_LAT),
        ] {
            if value.to_units(Rounding::Floor)?.0 < -limit
                || value.to_units(Rounding::Ceil)?.0 > limit
            {
                return Err(InvalidArea);
            }
        }
        Ok(boxed)
    }

    /// The box as four five-decimal strings joined with commas — the text the
    /// extract key is derived from (BR6.1, PD-1). Rendered from integers, so
    /// identical on every platform.
    pub fn key_text(&self) -> String {
        format!(
            "{},{},{},{}",
            format_units(self.min_lon),
            format_units(self.min_lat),
            format_units(self.max_lon),
            format_units(self.max_lat)
        )
    }
}

impl fmt::Display for CanonicalBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key_text())
    }
}

/// Render units as a decimal with exactly five places (`-79.63000`).
pub fn format_units(units: Units) -> String {
    let sign = if units.0 < 0 { "-" } else { "" };
    let magnitude = units.0.unsigned_abs();
    let scale = UNITS_PER_DEGREE.unsigned_abs();
    format!("{sign}{}.{:05}", magnitude / scale, magnitude % scale)
}

/// Nanodegrees to units, rounded toward negative infinity.
pub fn nano_to_units_floor(nano: i64) -> Units {
    Units(nano.div_euclid(NANO_PER_UNIT))
}

/// Nanodegrees to units, rounded toward positive infinity.
pub fn nano_to_units_ceil(nano: i64) -> Units {
    let floor = nano.div_euclid(NANO_PER_UNIT);
    Units(if nano.rem_euclid(NANO_PER_UNIT) == 0 {
        floor
    } else {
        floor + 1
    })
}

/// The outward-rounded extent of a set of points (a way's nodes), in units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent {
    pub min_lon: Units,
    pub min_lat: Units,
    pub max_lon: Units,
    pub max_lat: Units,
}

impl Extent {
    /// The extent of `(lon, lat)` points in nanodegrees; `None` when empty.
    pub fn of_points(points: impl IntoIterator<Item = (i64, i64)>) -> Option<Extent> {
        let mut extent: Option<Extent> = None;
        for (lon, lat) in points {
            let (lon_floor, lon_ceil) = (nano_to_units_floor(lon), nano_to_units_ceil(lon));
            let (lat_floor, lat_ceil) = (nano_to_units_floor(lat), nano_to_units_ceil(lat));
            extent = Some(match extent {
                None => Extent {
                    min_lon: lon_floor,
                    min_lat: lat_floor,
                    max_lon: lon_ceil,
                    max_lat: lat_ceil,
                },
                Some(e) => Extent {
                    min_lon: e.min_lon.min(lon_floor),
                    min_lat: e.min_lat.min(lat_floor),
                    max_lon: e.max_lon.max(lon_ceil),
                    max_lat: e.max_lat.max(lat_ceil),
                },
            });
        }
        extent
    }

    /// Whether this (closed) extent overlaps the box, whose maximum edges
    /// are exclusive — the same half-open reading the touched-cell span uses,
    /// so a way is found in a touched cell exactly when it intersects (BR5.2).
    pub fn intersects(&self, b: &CanonicalBox) -> bool {
        self.min_lon < b.max_lon
            && b.min_lon <= self.max_lon
            && self.min_lat < b.max_lat
            && b.min_lat <= self.max_lat
    }
}

#[derive(Clone, Copy)]
enum Rounding {
    Floor,
    Ceil,
}

/// A plain decimal, kept exact: `-?[0-9]+(\.[0-9]+)?`, never negative zero.
struct Decimal {
    negative: bool,
    integer: u64,
    fraction: Vec<u8>,
}

impl Decimal {
    fn parse(text: &str) -> Result<Decimal, InvalidArea> {
        let bytes = text.as_bytes();
        let (negative, rest) = match bytes.split_first() {
            Some((b'-', rest)) => (true, rest),
            _ => (false, bytes),
        };
        let dot = rest.iter().position(|b| *b == b'.');
        let (int_digits, frac_digits) = match dot {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        if int_digits.is_empty()
            || int_digits.len() > MAX_INTEGER_DIGITS
            || (dot.is_some() && frac_digits.is_empty())
        {
            return Err(InvalidArea);
        }
        if !int_digits.iter().chain(frac_digits).all(u8::is_ascii_digit) {
            return Err(InvalidArea);
        }
        let integer = int_digits
            .iter()
            .fold(0u64, |acc, d| acc * 10 + u64::from(d - b'0'));
        let fraction: Vec<u8> = frac_digits.iter().map(|d| d - b'0').collect();
        if negative && integer == 0 && fraction.iter().all(|d| *d == 0) {
            return Err(InvalidArea);
        }
        Ok(Decimal {
            negative,
            integer,
            fraction,
        })
    }

    fn to_units(&self, rounding: Rounding) -> Result<Units, InvalidArea> {
        let places = UNITS_PER_DEGREE.ilog10() as usize;
        let mut magnitude = self
            .integer
            .checked_mul(UNITS_PER_DEGREE.unsigned_abs())
            .ok_or(InvalidArea)?;
        for i in 0..places {
            let digit = self.fraction.get(i).copied().unwrap_or(0);
            magnitude += u64::from(digit) * 10u64.pow((places - 1 - i) as u32);
        }
        let remainder = self.fraction.iter().skip(places).any(|d| *d != 0);
        let away_from_zero = match rounding {
            Rounding::Floor => self.negative,
            Rounding::Ceil => !self.negative,
        };
        if remainder && away_from_zero {
            magnitude += 1;
        }
        let magnitude = i64::try_from(magnitude).map_err(|_| InvalidArea)?;
        Ok(Units(if self.negative { -magnitude } else { magnitude }))
    }

    fn cmp(&self, other: &Decimal) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let by_magnitude = self.integer.cmp(&other.integer).then_with(|| {
            let a = trim_zeros(&self.fraction);
            let b = trim_zeros(&other.fraction);
            a.cmp(b)
        });
        if self.negative {
            by_magnitude.reverse()
        } else {
            by_magnitude
        }
    }
}

fn trim_zeros(digits: &[u8]) -> &[u8] {
    let end = digits.iter().rposition(|d| *d != 0).map_or(0, |i| i + 1);
    &digits[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(text: &str) -> CanonicalBox {
        CanonicalBox::parse(text).expect("valid bbox")
    }

    // BR3.1 — four decimals in WGS84 order, min strictly below max, in range.
    #[test]
    fn parses_four_decimals_in_wgs84_order() {
        let b = canonical("-79.63000,43.65000,-79.62000,43.66000");
        assert_eq!(b.min_lon, Units(-7_963_000));
        assert_eq!(b.min_lat, Units(4_365_000));
        assert_eq!(b.max_lon, Units(-7_962_000));
        assert_eq!(b.max_lat, Units(4_366_000));
    }

    #[test]
    fn rejects_wrong_arity_and_empty() {
        for bad in ["", "1,2,3", "1,2,3,4,5", ",,,", "1,2,3,", "1,,3,4"] {
            assert_eq!(CanonicalBox::parse(bad), Err(InvalidArea), "{bad:?}");
        }
    }

    #[test]
    fn rejects_non_decimal_text() {
        // NaN, infinities, exponents, negative zero, signs, whitespace, hex:
        // none of these is a plain decimal (NFR6.4.1).
        for bad in [
            "NaN,1,2,3",
            "1,inf,2,3",
            "1,2,-inf,4",
            "1e1,2,3,4",
            "1,2E0,3,4",
            "-0,1,2,3",
            "-0.0,1,2,3",
            "+1,2,3,4",
            " 1,2,3,4",
            "1, 2,3,4",
            "0x1,2,3,4",
            ".5,1,2,3",
            "1.,1,2,3",
            "1..0,2,3,4",
            "a,b,c,d",
        ] {
            assert_eq!(CanonicalBox::parse(bad), Err(InvalidArea), "{bad:?}");
        }
    }

    #[test]
    fn rejects_out_of_range_coordinates() {
        assert_eq!(CanonicalBox::parse("-180.0000001,0,1,1"), Err(InvalidArea));
        assert_eq!(CanonicalBox::parse("0,0,180.0000001,1"), Err(InvalidArea));
        assert_eq!(CanonicalBox::parse("0,-90.00001,1,1"), Err(InvalidArea));
        assert_eq!(CanonicalBox::parse("0,0,1,90.00001"), Err(InvalidArea));
        // The exact limits are inside the range.
        let b = canonical("-180,-90,180,90");
        assert_eq!(b.min_lon, Units(-18_000_000));
        assert_eq!(b.max_lat, Units(9_000_000));
    }

    #[test]
    fn rejects_min_not_strictly_below_max() {
        assert_eq!(CanonicalBox::parse("1,1,1,2"), Err(InvalidArea));
        assert_eq!(CanonicalBox::parse("1,2,2,2"), Err(InvalidArea));
        assert_eq!(CanonicalBox::parse("2,1,1,2"), Err(InvalidArea));
        assert_eq!(
            CanonicalBox::parse("1.0000000,1,1.0000000,2"),
            Err(InvalidArea)
        );
        // Differing only beyond the canonical precision is still strictly below.
        let b = canonical("1.0000001,1,1.0000002,2");
        assert_eq!(b.min_lon, Units(100_000));
        assert_eq!(b.max_lon, Units(100_001));
    }

    // BR3.2 / PD-1 — five decimals, rounded outward: minimums floored,
    // maximums ceiled, so the canonical box never shrinks.
    #[test]
    fn canonicalises_outward_to_five_decimals() {
        let b = canonical("-79.631234567,43.6512345,-79.62000001,43.6600001");
        assert_eq!(b.min_lon, Units(-7_963_124)); // floor(-79.631234567e5)
        assert_eq!(b.min_lat, Units(4_365_123)); // floor(43.6512345e5)
        assert_eq!(b.max_lon, Units(-7_962_000)); // ceil(-79.62000001e5)
        assert_eq!(b.max_lat, Units(4_366_001)); // ceil(43.6600001e5)
    }

    #[test]
    fn exact_five_decimal_input_is_unchanged_by_canonicalisation() {
        let text = "-79.63000,43.65000,-79.62000,43.66000";
        assert_eq!(canonical(text).key_text(), text);
        assert_eq!(
            canonical("0.5,0.25,0.75,1").key_text(),
            "0.50000,0.25000,0.75000,1.00000"
        );
        assert_eq!(
            canonical("-0.5,-0.00001,0,0").key_text(),
            "-0.50000,-0.00001,0.00000,0.00000"
        );
    }

    #[test]
    fn equal_areas_written_differently_share_one_key_text() {
        assert_eq!(
            canonical("-79.63,43.65,-79.62,43.66").key_text(),
            canonical("-79.630000,43.6500,-79.6200000,43.66000").key_text()
        );
    }

    #[test]
    fn random_strings_never_panic() {
        // A small deterministic PRNG so the property holds on every run
        // without a proptest dependency (NFR6.4.1).
        let alphabet: &[u8] = b"0123456789.,-+eEinfNa x";
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            let mut text = String::new();
            let len = (state % 24) as usize;
            for _ in 0..len {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let index = (state % alphabet.len() as u64) as usize;
                text.push(alphabet[index] as char);
            }
            if let Ok(b) = CanonicalBox::parse(&text) {
                assert!(b.min_lon < b.max_lon && b.min_lat < b.max_lat);
            }
        }
    }

    #[test]
    fn extent_of_points_rounds_outward() {
        let extent = Extent::of_points([
            (-796_312_345_678, 436_512_345_678),
            (-796_200_000_000, 436_600_000_000),
        ])
        .expect("two points");
        assert_eq!(extent.min_lon, Units(-79_631_235));
        assert_eq!(extent.min_lat, Units(43_651_234));
        assert_eq!(extent.max_lon, Units(-79_620_000));
        assert_eq!(extent.max_lat, Units(43_660_000));
        assert!(Extent::of_points(std::iter::empty()).is_none());
    }

    #[test]
    fn extent_intersects_box_with_half_open_box_edges() {
        let b = canonical("0.02,0.02,0.03,0.03");
        let touching_left_edge =
            Extent::of_points([(15_000_000, 25_000_000), (20_000_000, 25_000_000)]).unwrap();
        assert!(touching_left_edge.intersects(&b));
        let starting_at_right_edge =
            Extent::of_points([(30_000_000, 25_000_000), (35_000_000, 25_000_000)]).unwrap();
        assert!(!starting_at_right_edge.intersects(&b));
        let inside = Extent::of_points([(25_000_000, 25_000_000)]).unwrap();
        assert!(inside.intersects(&b));
        let far = Extent::of_points([(90_000_000, 90_000_000)]).unwrap();
        assert!(!far.intersects(&b));
    }
}
