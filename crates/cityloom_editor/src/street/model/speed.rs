//! How a street writes its speed limit (`cityloom_model::street::Speed`).

#[cfg(test)]
mod tests {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Speed, Street, StreetExt};

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
}
