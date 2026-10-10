//! Finding a place and the roads in it: what is asked of Nominatim and Overpass,
//! and how their answers are read. Pure, so it is tested without a network.

pub mod area;
pub mod loader;
pub mod nominatim;
pub mod overpass;
pub mod tiles;
pub mod view;
pub mod vm;

use crate::shared::i18n::{I18n, Resources};
use crate::shared::said::Said;
use crate::shared::units::Units;

/// What the place search says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// A place's message in words, for a view. Nothing a place says has a length in it, so the units do not matter.
pub fn say(i18n: &I18n, said: &Said) -> String {
    crate::shared::said::say(i18n, Units::Metres, said)
}

/// The same, for a command: nothing is watched.
pub fn say_now(i18n: &I18n, said: &Said) -> String {
    crate::shared::said::say_now(i18n, Units::Metres, said)
}

/// A box on the earth, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

impl Bounds {
    /// The box `half_m` metres either way of a point.
    pub fn around(lat: f64, lon: f64, half_m: f64) -> Bounds {
        const M_PER_DEG_LAT: f64 = 111_320.0;
        let dlat = half_m / M_PER_DEG_LAT;
        let dlon = half_m / (M_PER_DEG_LAT * lat.to_radians().cos().max(0.01));
        Bounds { south: lat - dlat, west: lon - dlon, north: lat + dlat, east: lon + dlon }
    }
}

/// Percent-encodes `text` for a URL query value.
pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The text a percent-encoded query value stands for; anything not valid is passed through as it is.
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (bytes[i], bytes.get(i + 1).copied().and_then(hex), bytes.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b'+', ..) => {
                out.push(b' ');
                i += 1;
            }
            (b, ..) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_around_a_point_is_as_wide_as_it_is_high_in_metres() {
        let b = Bounds::around(51.5, -0.12, 500.0);
        let high = (b.north - b.south) * 111_320.0;
        let wide = (b.east - b.west) * 111_320.0 * 51.5_f64.to_radians().cos();
        assert!((high - 1000.0).abs() < 1.0 && (wide - 1000.0).abs() < 1.0, "{high} x {wide}");
        assert!(b.south < 51.5 && 51.5 < b.north && b.west < -0.12 && -0.12 < b.east);
    }

    #[test]
    fn encoded_text_is_decoded_and_what_is_not_valid_is_left() {
        assert_eq!(decode("Rue%20de%20l%27%C3%89glise%2C+Paris"), "Rue de l'Église, Paris");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
    }

    #[test]
    fn the_place_ftl_files_have_the_same_messages_and_variables() {
        use crate::shared::i18n::tests::parity_problems;
        assert_eq!(parity_problems(RESOURCES.en, RESOURCES.fr), Vec::<String>::new());
    }

    #[test]
    fn a_load_failure_is_worded_in_the_language_the_page_starts_in() {
        use crate::shared::i18n::{Locale, detect};
        use crate::shared::ports::test_ports_with_page;
        let (ports, page, _) = test_ports_with_page();
        *page.language.borrow_mut() = Some("fr-CA".into());
        let failure = || Err(Said::new("place-roads-not-kept"));
        let words = |locale| problem_in_words(&crate::i18n_for(locale), failure());
        assert_eq!(detect(&ports), Locale::FrCa);
        assert_eq!(words(detect(&ports)), "les rues n’ont pas pu être conservées\u{a0}: le stockage est bloqué ou plein");
        assert_eq!(words(Locale::En), "the roads could not be kept: storage is blocked or full");
        assert_eq!(problem_in_words(&crate::i18n_for(detect(&ports)), Ok(())), "");
    }

    #[test]
    fn text_is_encoded_for_a_query() {
        assert_eq!(encode("Rue de l'Église, Paris"), "Rue%20de%20l%27%C3%89glise%2C%20Paris");
        assert_eq!(encode("a-b_c.d~e9"), "a-b_c.d~e9");
    }
}

/// What a load went wrong with, in words; the empty string when it went well.
pub fn problem_in_words(i18n: &I18n, result: Result<(), Said>) -> String {
    result.err().map(|why| say_now(i18n, &why)).unwrap_or_default()
}

/// Gets the roads of the area the person is working in, from where they come from, unless
/// they are already kept. The pages call this before they open the city. Resolves with the
/// empty string when the city is ready, or with what went wrong, in words; the city is then the
/// sample, so the page still works.
///
/// What went wrong is worded in the language the page starts in: the stored choice, else the browser's
/// (`detect`, which only reads; the page sets `<html lang>` itself later).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn prepare_city() -> js_sys::Promise {
    use crate::city::store::CityStore;
    use crate::shared::platform::browser_ports;
    js_sys::Promise::new(&mut |resolve, _| {
        let ports = browser_ports();
        let store = CityStore::current(ports.storage.clone());
        let announcer = ports.announcer.clone();
        let i18n = crate::i18n_for(crate::shared::i18n::detect(&ports));
        loader::Loader::new(ports).load(&store, move |result| {
            let problem = problem_in_words(&i18n, result);
            if !problem.is_empty() {
                announcer.say(&problem);
            }
            let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(&problem));
        });
    })
}
