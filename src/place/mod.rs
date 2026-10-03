//! Finding a place and the roads in it: what is asked of Nominatim and Overpass,
//! and how their answers are read. Pure, so it is tested without a network.

pub mod area;
pub mod loader;
pub mod nominatim;
pub mod overpass;
pub mod vm;

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
    fn text_is_encoded_for_a_query() {
        assert_eq!(encode("Rue de l'Église, Paris"), "Rue%20de%20l%27%C3%89glise%2C%20Paris");
        assert_eq!(encode("a-b_c.d~e9"), "a-b_c.d~e9");
    }
}

/// Gets the roads of the area the person is working in, from where they come from, unless
/// they are already kept. The pages call this before they open the city. Resolves with the
/// empty string when the city is ready, or with what went wrong, in words; the city is then the
/// sample, so the page still works.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn prepare_city() -> js_sys::Promise {
    use crate::city::store::CityStore;
    use crate::shared::platform::browser_ports;
    js_sys::Promise::new(&mut |resolve, _| {
        let ports = browser_ports();
        let Some(area) = CityStore::current_area(&*ports.storage) else {
            let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(""));
            return;
        };
        let store = CityStore::for_area(ports.storage.clone(), area);
        let announcer = ports.announcer.clone();
        loader::Loader::new(ports).load(&store, move |result| {
            let problem = result.err().unwrap_or_default();
            if !problem.is_empty() {
                announcer.say(&problem);
            }
            let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(&problem));
        });
    })
}
