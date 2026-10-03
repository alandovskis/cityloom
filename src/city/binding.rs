//! A place in the city that a page edits, and how what it makes is written back.

use crate::city::store::CityStore;
use crate::junction::model::State;
use crate::street::model::Street;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Junction(u32),
    Street(u32),
}

/// The page edits a junction or a street of the city kept in `store`.
#[derive(Clone)]
pub struct CityBinding {
    pub store: CityStore,
    pub place: Place,
}

impl CityBinding {
    /// Writes a junction as it now is back to the city. Says whether it was kept.
    pub fn keep_junction(&self, state: State) -> bool {
        let Place::Junction(node) = self.place else { return false };
        self.store.write(|c| c.keep_junction(node, state))
    }

    /// Writes a street as it now is back to the city. Says whether it was kept.
    pub fn keep_street(&self, street: Street) -> bool {
        let Place::Street(edge) = self.place else { return false };
        self.store.write(|c| c.keep_street(edge, street))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::test_ports;

    fn binding(place: impl Fn(&crate::city::model::CityView) -> Place) -> CityBinding {
        let (ports, ..) = test_ports();
        let store = CityStore::new(ports.storage);
        let place = place(&store.open().view(0));
        CityBinding { store, place }
    }

    #[test]
    fn a_street_is_written_back_and_found_by_the_next_page() {
        let b = binding(|v| Place::Street(v.edges[0].uid));
        let Place::Street(edge) = b.place else { unreachable!() };
        let mut e = b.store.open().street_editor(edge, 0).unwrap();
        let u = e.view().segments[0].uid;
        e.nudge_width(u, -100);
        assert!(b.keep_street(e.snapshot()));
        assert_eq!(b.store.open().view(0).edited, 1);
        assert!(!b.keep_junction(crate::junction::model::Junction::new(0).snapshot()), "not a junction");
    }

    #[test]
    fn a_junction_is_written_back_and_a_street_is_not_taken_for_one() {
        let b = binding(|v| Place::Junction(v.nodes.iter().find(|n| n.junction).unwrap().uid));
        let Place::Junction(node) = b.place else { unreachable!() };
        let mut j = b.store.open().junction_editor(node, 0).unwrap();
        let uid = j.current().arms[0].uid;
        assert!(j.set_corner(uid, 7_000));
        assert!(b.keep_junction(j.snapshot()));
        assert_eq!(b.store.open().view(0).edited, 1);
        let mut e = crate::street::model::Editor::new(0);
        let u = e.view().segments[0].uid;
        e.nudge_width(u, 100);
        assert!(!b.keep_street(e.snapshot()));
    }
}
