//! Area restrictions: a traffic rule over an outline the resident draws on the map, kept in its own document, and
//! the streets inside checked against it. The model, the drawing and the document live here; the map page's tab
//! is the map slice's (`map::areas`), and the street page reads the model and the store.

pub mod model;

use crate::shared::i18n::Resources;

/// What an area restriction says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// Bendville, the one-street city the slice's tests (and the map's and street's restriction tests) are written
/// against.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::rc::Rc;

    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};

    use crate::city::projection::{self, Projection};
    use crate::city::store::CityStore;
    use crate::place::area::Area;
    use crate::restriction::model::{LonLat, midpoint};
    use crate::shared::ports::Storage;

    /// One street of two driving lanes that bends: east for 80 m, then north-east to 100 m east and 50 m north.
    pub fn bent_network() -> Network {
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], osm_versions: Default::default(), x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0, hours: None };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                osm_versions: Default::default(),
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        }
    }

    pub fn bendville() -> Area {
        Area::new("Bendville", 45.5, -73.6)
    }

    /// Bendville's roads kept in `storage` and chosen, so `CityStore::current` opens it.
    pub fn bent_store(storage: Rc<dyn Storage>) -> CityStore {
        let store = CityStore::for_area(storage.clone(), bendville());
        assert!(store.keep_network(&bent_network()));
        assert!(CityStore::choose(&*storage, &bendville()));
        store
    }

    /// Where Bendville lies, and the longitude and latitude of its street's midpoint.
    pub fn bent_midpoint(store: &CityStore) -> (Projection, LonLat) {
        let v = store.open().view(0);
        let p = projection::for_store(store, v.origin_m).expect("Bendville's roads are kept");
        let [x, y] = midpoint(&v.edges[0].shape_mm).expect("the street has a shape");
        (p, p.lon_lat(x.round() as i32, y.round() as i32))
    }

    /// A square `half` degrees either way of `p`, its corners anticlockwise from the south-west.
    pub fn square(p: LonLat, half: f64) -> Vec<LonLat> {
        vec![[p[0] - half, p[1] - half], [p[0] + half, p[1] - half], [p[0] + half, p[1] + half], [p[0] - half, p[1] + half]]
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::i18n::tests::parity_problems;

    #[test]
    fn the_restriction_ftl_files_have_the_same_messages_and_variables() {
        assert_eq!(parity_problems(super::RESOURCES.en, super::RESOURCES.fr), Vec::<String>::new());
    }
}
