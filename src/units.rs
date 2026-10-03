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

    /// A length in millimetres to `places` decimals, for a field to hold.
    pub fn fixed(self, mm: i32, places: usize) -> String {
        let per = match self {
            Units::Metres => 1000.0,
            Units::Feet => MM_PER_FOOT,
        };
        format!("{:.*}", places, mm as f64 / per)
    }

    /// Whole millimetres for a number typed in these units.
    pub fn mm(self, typed: f64) -> i32 {
        let per = match self {
            Units::Metres => 1000.0,
            Units::Feet => MM_PER_FOOT,
        };
        (typed * per).round() as i32
    }

    /// A length as a street shows it: to a tenth of a metre, or to a hundredth
    /// when it is not a whole tenth; feet to a tenth.
    pub fn fine(self, mm: i32) -> String {
        match self {
            Units::Metres if mm % 100 != 0 => self.fixed(mm, 2),
            _ => self.number(mm),
        }
    }

    /// A street length with its unit: 3.3 m.
    pub fn length_fine(self, mm: i32) -> String {
        format!("{} {}", self.fine(mm), self.word())
    }

    /// A change in length with its sign, `+1.2` or `\u{2212}1.2`, and `0` for none.
    pub fn signed(self, mm: i32) -> String {
        match mm {
            0 => "0".to_string(),
            _ => format!("{}{}", if mm > 0 { "+" } else { "\u{2212}" }, self.fine(mm.abs())),
        }
    }

    /// What the units are called in a sentence.
    pub fn name(self) -> &'static str {
        match self {
            Units::Metres => "metres",
            Units::Feet => "feet",
        }
    }

    /// How far a width nudge moves a piece: a tenth of a metre, or about a foot.
    pub fn step_mm(self) -> i32 {
        match self {
            Units::Metres => 100,
            Units::Feet => 305,
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

    #[test]
    fn an_input_shows_a_length_to_the_places_it_asks_for() {
        assert_eq!(Units::Metres.fixed(11_400, 1), "11.4");
        assert_eq!(Units::Metres.fixed(-100, 2), "-0.10");
        assert_eq!(Units::Metres.fixed(0, 2), "0.00");
        assert_eq!(Units::Feet.fixed(3_048, 1), "10.0");
        assert_eq!(Units::Feet.fixed(100, 2), "0.33");
    }

    #[test]
    fn a_typed_number_becomes_whole_millimetres() {
        assert_eq!(Units::Metres.mm(11.4), 11_400);
        assert_eq!(Units::Metres.mm(0.1234), 123);
        assert_eq!(Units::Metres.mm(-0.1), -100);
        assert_eq!(Units::Feet.mm(10.0), 3_048);
        assert_eq!(Units::Feet.mm(0.5), 152);
    }

    #[test]
    fn a_street_length_shows_a_tenth_or_a_hundredth_when_it_needs_one() {
        assert_eq!(Units::Metres.fine(3_000), "3.0");
        assert_eq!(Units::Metres.fine(2_700), "2.7");
        assert_eq!(Units::Metres.fine(2_750), "2.75");
        // 0.305 is a hair under in binary, and rounds down, as the page always did.
        assert_eq!(Units::Metres.fine(305), "0.30");
        assert_eq!(Units::Feet.fine(3_048), "10.0");
        assert_eq!(Units::Feet.fine(2_750), "9.0");
    }

    #[test]
    fn a_change_in_length_has_its_sign_and_zero_has_none() {
        assert_eq!(Units::Metres.signed(0), "0");
        assert_eq!(Units::Metres.signed(1_200), "+1.2");
        assert_eq!(Units::Metres.signed(-1_200), "\u{2212}1.2");
        assert_eq!(Units::Metres.signed(-250), "\u{2212}0.25");
        assert_eq!(Units::Feet.signed(3_048), "+10.0");
    }

    #[test]
    fn the_units_are_named_in_words_and_have_a_step() {
        assert_eq!(Units::Metres.name(), "metres");
        assert_eq!(Units::Feet.name(), "feet");
        assert_eq!(Units::Metres.step_mm(), 100);
        assert_eq!(Units::Feet.step_mm(), 305);
    }
}
