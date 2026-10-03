//! Lengths as the resident reads them: metres or feet, to one decimal.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Units {
    #[default]
    Metres,
    Feet,
}

const MM_PER_FOOT: f64 = 304.8;

impl Units {
    /// `"ft"` is feet; anything else is metres.
    pub fn parse(word: &str) -> Units {
        if word == "ft" { Units::Feet } else { Units::Metres }
    }

    pub fn word(self) -> &'static str {
        match self {
            Units::Metres => "m",
            Units::Feet => "ft",
        }
    }

    /// A length in millimetres without its unit, to a tenth: 11.4.
    pub fn number(self, mm: i32) -> String {
        match self {
            Units::Metres => {
                // Whole tenths of a metre, a tie going away from zero.
                let tenths = (mm.unsigned_abs() + 50) / 100;
                let sign = if mm < 0 && tenths > 0 { "-" } else { "" };
                format!("{sign}{}.{}", tenths / 10, tenths % 10)
            }
            Units::Feet => format!("{:.1}", mm as f64 / MM_PER_FOOT),
        }
    }

    /// A length with its unit: 11.4 m.
    pub fn length(self, mm: i32) -> String {
        format!("{} {}", self.number(mm), self.word())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metres_are_written_to_a_tenth() {
        assert_eq!(Units::Metres.number(11_400), "11.4");
        assert_eq!(Units::Metres.number(3_000), "3.0");
        assert_eq!(Units::Metres.number(0), "0.0");
        assert_eq!(Units::Metres.number(36_000), "36.0");
    }

    #[test]
    fn a_tie_between_two_tenths_goes_away_from_zero() {
        assert_eq!(Units::Metres.number(150), "0.2");
        assert_eq!(Units::Metres.number(-150), "-0.2");
        assert_eq!(Units::Metres.number(149), "0.1");
    }

    #[test]
    fn a_negative_length_keeps_its_sign_and_zero_has_none() {
        assert_eq!(Units::Metres.number(-450), "-0.5");
        assert_eq!(Units::Metres.number(-10), "0.0");
    }

    #[test]
    fn feet_are_written_to_a_tenth() {
        assert_eq!(Units::Feet.number(3_048), "10.0");
        assert_eq!(Units::Feet.number(11_400), "37.4");
        assert_eq!(Units::Feet.number(0), "0.0");
    }

    #[test]
    fn a_length_carries_its_unit() {
        assert_eq!(Units::Metres.length(11_400), "11.4 m");
        assert_eq!(Units::Feet.length(3_048), "10.0 ft");
    }

    #[test]
    fn units_are_read_from_the_page_s_word() {
        assert_eq!(Units::parse("ft"), Units::Feet);
        assert_eq!(Units::parse("m"), Units::Metres);
        assert_eq!(Units::parse("cubits"), Units::Metres);
        assert_eq!(Units::Feet.word(), "ft");
        assert_eq!(Units::Metres.word(), "m");
    }
}
