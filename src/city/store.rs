//! The city the street, junction and map pages share, as it is kept between
//! visits. A page opens it, takes out what it edits, and writes back what it
//! made, against the city as it is in storage now so that a change made in
//! another tab is not written over.

use std::rc::Rc;

use crate::city::model::City;
use crate::shared::ports::Storage;

pub const CITY_KEY: &str = "cityloom-city";
pub const REGION_KEY: &str = "cityloom-region";

#[derive(Clone)]
pub struct CityStore {
    storage: Rc<dyn Storage>,
}

impl CityStore {
    pub fn new(storage: Rc<dyn Storage>) -> CityStore {
        CityStore { storage }
    }

    /// The city as it is kept, or as first laid out when nothing usable is.
    pub fn open(&self) -> City {
        City::load(&self.storage.recall(CITY_KEY).unwrap_or_default())
    }

    /// Applies `change` to the city as it is kept now and keeps the result.
    /// Says whether the change was taken and kept.
    pub fn write(&self, change: impl FnOnce(&mut City) -> bool) -> bool {
        let mut city = self.open();
        change(&mut city) && self.storage.remember(CITY_KEY, &city.save())
    }

    /// Puts every street and junction back as first laid out, and keeps that.
    pub fn reset(&self) -> bool {
        self.write(|city| {
            city.reset();
            true
        })
    }

    /// The index of the region the person chose, which the pages share; the
    /// first when none is chosen or it is unknown.
    pub fn region(&self) -> usize {
        let id = self.storage.recall(REGION_KEY);
        crate::shared::catalogue::REGIONS.iter().position(|r| Some(r.id) == id.as_deref()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::test_ports;

    fn store() -> (CityStore, Rc<crate::shared::ports::MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        (CityStore::new(ports.storage), storage)
    }

    #[test]
    fn a_new_visit_opens_the_city_as_first_laid_out() {
        let (s, _) = store();
        let v = s.open().view(0);
        assert_eq!(v.edited, 0);
        assert!(v.places > 0);
    }

    #[test]
    fn what_a_page_writes_is_kept_and_found_by_the_next() {
        let (s, _) = store();
        let street = s.open().view(0).edges[0].uid;
        assert!(s.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            e.nudge_width(u, 100);
            c.keep_street(street, e.snapshot())
        }));
        assert_eq!(s.open().view(0).edited, 1);
    }

    #[test]
    fn a_change_that_is_not_taken_is_not_kept() {
        let (s, st) = store();
        assert!(!s.write(|_| false));
        assert_eq!(st.recall(CITY_KEY), None);
    }

    #[test]
    fn a_blocked_storage_says_the_change_was_not_kept() {
        let (s, st) = store();
        st.blocked(true);
        assert!(!s.write(|_| true));
    }

    #[test]
    fn a_page_writes_against_what_another_tab_kept_not_over_it() {
        let (s, _) = store();
        let v = s.open().view(0);
        let (a, b) = (v.edges[0].uid, v.edges[1].uid);
        let nudge = |street: u32| {
            move |c: &mut City| {
                let mut e = c.street_editor(street, 0).unwrap();
                let u = e.view().segments[0].uid;
                e.nudge_width(u, 100);
                c.keep_street(street, e.snapshot())
            }
        };
        // Two pages opened the city at the same time; each writes its own street.
        assert!(s.write(nudge(a)));
        assert!(s.write(nudge(b)));
        assert_eq!(s.open().view(0).edited, 2);
    }

    #[test]
    fn starting_over_puts_everything_back_and_keeps_that() {
        let (s, _) = store();
        let street = s.open().view(0).edges[0].uid;
        s.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            e.nudge_width(u, 100);
            c.keep_street(street, e.snapshot())
        });
        assert_eq!(s.open().view(0).edited, 1);
        assert!(s.reset());
        assert_eq!(s.open().view(0).edited, 0);
    }

    #[test]
    fn the_chosen_region_is_read_from_storage_with_the_first_as_the_default() {
        let (s, st) = store();
        assert_eq!(s.region(), 0);
        st.remember(REGION_KEY, "united-kingdom");
        assert_eq!(s.region(), 3);
        st.remember(REGION_KEY, "nowhere");
        assert_eq!(s.region(), 0);
    }
}
