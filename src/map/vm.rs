//! The city map's view-model: how the city reads as places to open, what needs
//! attention and what has changed, where the camera is, and what pressing
//! "start over" means. The view binds to its properties and sends it commands.

use std::cell::Cell;
use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::city::model::{City, CityView, EdgeView, NodeView};
use crate::city::store::CityStore;
use crate::map::camera::{Camera, Insets, World};
use crate::map::overlay;
use crate::map::projection::Projection;
use crate::map::style;
use crate::shared::core::{Core, Presents};
use crate::shared::ports::{MapEvent, Ports};
use crate::shared::units::Units;

pub const ARMED_RESET: &str = "Press again to start over";
pub const RESET: &str = "Start over";
const ARMED_SAID: &str = "This puts every street and junction back as first laid out. Press again to confirm.";
const DONE_SAID: &str = "The city is back as it was first laid out.";

/// How far a zoom button, or a key, zooms at once.
const ZOOM_STEP: f64 = 1.4;
/// How far an arrow key pans, in pixels.
const PAN_PX: f64 = 80.0;

/// What the map page says where it has no map to show.
pub const MISSING: &str = "The basemap could not be loaded. Build it with `just prepare`, then reload the page.";
pub const OUTSIDE: &str = "There is no basemap for this place. The map covers the Montréal area.";
pub const NO_ROADS: &str = "The roads of this place could not be loaded, so there is no map to show.";
/// What the status line says of a city with no places.
pub const NOTHING_TO_SHOW: &str = "No roads to show.";

/// Whether the basemap is there to draw the places on.
#[derive(Clone, Debug, PartialEq)]
pub enum BasemapState {
    /// Asked for; the tiles have not answered.
    Waiting,
    Ready,
    /// There will be no map, for the reason in these words.
    Unavailable(&'static str),
}

/// The place a `hot` name (`s-7`, `j-3`) opens.
fn href_of(hot: &str) -> Option<String> {
    let (kind, uid) = hot.split_once('-')?;
    let uid: u32 = uid.parse().ok()?;
    match kind {
        "s" => Some(street_href(uid)),
        "j" => Some(junction_href(uid)),
        _ => None,
    }
}

struct CityModel {
    city: City,
    region: usize,
}

impl Presents for CityModel {
    type View = CityView;
    fn present(&self) -> CityView {
        self.city.view(self.region)
    }
}

// ---- words ------------------------------------------------------------------------------

/// `1 street`, `3 streets`.
pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn streets(n: usize) -> String {
    plural(n, "street", "streets")
}

/// The places a street runs between, from its name `Kind · Here and there`.
pub fn street_ends(name: &str) -> &str {
    name.split(" \u{b7} ").nth(1).unwrap_or("")
}

/// What a place needs, or that it is changed, for a screen reader.
fn state_words(ok: bool, failing: &[String], edited: bool) -> String {
    let state = if ok { String::new() } else { format!(" Needs attention: {}.", failing.join(", ").to_lowercase()) };
    format!("{state}{}", if edited { " Changed." } else { "" })
}

/// How a junction on the map is told to a screen reader.
pub fn junction_label(n: &NodeView) -> String {
    format!("{}, {}, {}.{} Opens the junction plan.", n.name, n.control.unwrap_or("").to_lowercase(), streets(n.arms), state_words(n.ok, &n.failing, n.edited))
}

/// How a street on the map is told to a screen reader.
pub fn street_label(e: &EdgeView, units: Units) -> String {
    format!("{}, {}, {} wide.{} Opens the street cross-section.", e.kind, street_ends(&e.name), units.length(e.row_mm), state_words(e.ok, &e.failing, e.edited))
}

pub fn street_href(uid: u32) -> String {
    format!("street.html?street={uid}")
}

pub fn junction_href(uid: u32) -> String {
    format!("intersection.html?junction={uid}")
}

// ---- what the view binds to ----------------------------------------------------------------

/// A place to open: in a list, or as a link on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceRow {
    pub href: String,
    /// What the map and the list highlight together: `j-3` or `s-7`.
    pub hot: String,
    pub name: String,
    pub sub: String,
    pub tag: Option<StateTag>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateTag {
    pub text: &'static str,
    pub bad: bool,
}

/// A place in the notes: a link and one line about it.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteItem {
    pub href: String,
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub text: String,
    pub bad: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetOutcome {
    /// Nothing has changed, so there is nothing to start over from.
    Ignored,
    /// Pressed once: it asks to be pressed again.
    Armed,
    Done,
}

pub struct MapVm {
    me: Weak<MapVm>,
    core: Core<CityModel>,
    store: CityStore,
    ports: Ports,
    camera: ArcRwSignal<Camera>,
    basemap: ArcRwSignal<BasemapState>,
    moved: Cell<bool>,
    hot: ArcRwSignal<Option<String>>,
    armed: ArcRwSignal<bool>,
    search: ArcRwSignal<String>,
    /// The result the arrow keys have moved to, by index into `results`.
    active: ArcRwSignal<Option<usize>>,
}

impl MapVm {
    /// The city of the area chosen. When that area's roads could not be had it is a city of no places,
    /// named after the area.
    pub fn new(ports: Ports) -> Rc<MapVm> {
        let store = CityStore::current(ports.storage.clone());
        MapVm::over(store, ports)
    }

    /// In the tests: the page over the hand-made city they are written against.
    #[cfg(test)]
    pub fn on_sample(ports: Ports) -> Rc<MapVm> {
        let store = CityStore::sample(ports.storage.clone());
        MapVm::over(store, ports)
    }

    fn over(store: CityStore, ports: Ports) -> Rc<MapVm> {
        let has_roads = store.has_network();
        let model = CityModel { city: store.open(), region: store.region() };
        let core = Core::new(model);
        let world = World::round(core.view_now().bounds_mm);
        let basemap = if has_roads { BasemapState::Waiting } else { BasemapState::Unavailable(NO_ROADS) };
        Rc::new_cyclic(|me| MapVm {
            me: me.clone(),
            core,
            store,
            ports,
            camera: ArcRwSignal::new(Camera::new(world, 800.0, 520.0)),
            basemap: ArcRwSignal::new(basemap),
            moved: Cell::new(false),
            hot: ArcRwSignal::new(None),
            armed: ArcRwSignal::new(false),
            search: ArcRwSignal::new(String::new()),
            active: ArcRwSignal::new(None),
        })
    }

    // ---- properties ----

    pub fn view(&self) -> Rc<CityView> {
        self.core.view()
    }

    pub fn units(&self) -> Units {
        self.core.units()
    }

    pub fn title(&self) -> String {
        self.view().name.to_string()
    }

    fn junctions(v: &CityView) -> Vec<&NodeView> {
        let mut j: Vec<&NodeView> = v.nodes.iter().filter(|n| n.junction).collect();
        j.sort_by_key(|n| n.number);
        j
    }

    /// `9 junctions, 32 streets`.
    pub fn counts(&self) -> String {
        let v = self.view();
        format!("{}, {}", plural(Self::junctions(&v).len(), "junction", "junctions"), streets(v.edges.len()))
    }

    pub fn places(&self) -> usize {
        self.view().places
    }

    /// A city with nothing in it: an area whose roads could not be had, or that has none.
    fn has_no_places(&self) -> bool {
        self.places() == 0
    }

    pub fn changes(&self) -> usize {
        self.view().edited
    }

    fn tag(ok: bool, edited: bool) -> Option<StateTag> {
        if !ok { Some(StateTag { text: "Needs attention", bad: true }) } else { edited.then_some(StateTag { text: "Changed", bad: false }) }
    }

    /// The junctions, in the order of their numbers, as rows to open.
    pub fn junction_rows(&self) -> Vec<PlaceRow> {
        let v = self.view();
        Self::junctions(&v)
            .into_iter()
            .map(|n| PlaceRow {
                href: junction_href(n.uid),
                hot: format!("j-{}", n.uid),
                name: n.name.clone(),
                sub: format!("{}, {}", n.control.unwrap_or(""), streets(n.arms)),
                tag: Self::tag(n.ok, n.edited),
            })
            .collect()
    }

    pub fn street_rows(&self) -> Vec<PlaceRow> {
        let (v, units) = (self.view(), self.units());
        v.edges
            .iter()
            .map(|e| PlaceRow {
                href: street_href(e.uid),
                hot: format!("s-{}", e.uid),
                name: e.kind.to_string(),
                sub: format!("{} \u{b7} {}", street_ends(&e.name), units.length(e.row_mm)),
                tag: Self::tag(e.ok, e.edited),
            })
            .collect()
    }

    // ---- search ----

    /// What was typed in the search box.
    pub fn search_text(&self) -> String {
        self.search.get()
    }

    pub fn set_search(&self, text: &str) {
        self.search.set(text.to_string());
        self.active.set(None);
    }

    /// The places the search finds: the junctions, then the streets.
    pub fn results(&self) -> Vec<PlaceRow> {
        if self.terms().is_empty() {
            return Vec::new();
        }
        let (j, s) = self.narrowed();
        j.into_iter().chain(s).collect()
    }

    /// How many results the dropdown shows before it says how many more there are.
    pub const SHOWN: usize = 8;

    /// The result the arrow keys are on.
    pub fn active_result(&self) -> Option<usize> {
        self.active.get()
    }

    /// Moves down (1) or up (-1) the results shown, and stops at the ends.
    pub fn move_active(&self, by: i32) {
        let n = self.results().len().min(Self::SHOWN) as i32;
        if n == 0 {
            return;
        }
        let at = self.active.get_untracked().map_or(-1, |i| i as i32);
        self.active.set(Some((at + by).clamp(0, n - 1) as usize));
    }

    /// The words searched for, lower case.
    fn terms(&self) -> Vec<String> {
        self.search.get().to_lowercase().split_whitespace().map(String::from).collect()
    }

    /// The junctions and the streets that the search leaves. The whole phrase is
    /// looked for in a place's name and small print first, so that "junction 4"
    /// finds Junction 4 and the streets that end there and not every junction of
    /// four streets; when that finds nothing, each word is looked for on its own.
    fn narrowed(&self) -> (Vec<PlaceRow>, Vec<PlaceRow>) {
        let terms = self.terms();
        let (junctions, streets) = (self.junction_rows(), self.street_rows());
        let text = |r: &PlaceRow| format!("{} {}", r.name, r.sub).to_lowercase();
        let phrase = terms.join(" ");
        let by_phrase = |rows: &[PlaceRow]| rows.iter().filter(|r| text(r).contains(&phrase)).cloned().collect::<Vec<_>>();
        let by_words = |rows: &[PlaceRow]| rows.iter().filter(|r| terms.iter().all(|t| text(r).contains(t.as_str()))).cloned().collect::<Vec<_>>();
        let (j, st) = (by_phrase(&junctions), by_phrase(&streets));
        if j.is_empty() && st.is_empty() { (by_words(&junctions), by_words(&streets)) } else { (j, st) }
    }

    /// How the search came out, or nothing when nothing is searched for.
    pub fn search_note(&self) -> Option<String> {
        if self.terms().is_empty() {
            return None;
        }
        if self.has_no_places() {
            return Some("There are no places to search.".to_string());
        }
        let n = self.results().len();
        Some(if n == 0 {
            format!("No places match \u{201c}{}\u{201d}. Clear the search to see all {}.", self.search.get().trim(), self.places())
        } else if n > Self::SHOWN {
            format!("{} \u{b7} showing the first {}", plural(n, "place matches", "places match"), Self::SHOWN)
        } else {
            plural(n, "place matches", "places match")
        })
    }

    /// Where pressing Enter goes: the result the arrow keys are on, else the first.
    pub fn chosen_href(&self) -> Option<String> {
        let results = self.results();
        let at = self.active.get_untracked().unwrap_or(0).min(Self::SHOWN - 1);
        results.into_iter().nth(at).map(|r| r.href)
    }

    /// How a junction is told on the map.
    pub fn junction_label(&self, uid: u32) -> String {
        self.view().nodes.iter().find(|n| n.uid == uid).map(junction_label).unwrap_or_default()
    }

    pub fn street_label(&self, uid: u32) -> String {
        let units = self.units();
        self.view().edges.iter().find(|e| e.uid == uid).map(|e| street_label(e, units)).unwrap_or_default()
    }

    /// Every place, as the notes list them: junctions, then streets.
    fn places_for_notes(v: &CityView) -> Vec<(NoteItem, bool, bool, &[String])> {
        let mut all = Vec::new();
        for n in Self::junctions(v) {
            all.push((NoteItem { href: junction_href(n.uid), name: n.name.clone(), detail: String::new() }, n.ok, n.edited, n.failing.as_slice()));
        }
        for e in &v.edges {
            all.push((
                NoteItem { href: street_href(e.uid), name: format!("{}, {}", e.kind, street_ends(&e.name)), detail: String::new() },
                e.ok,
                e.edited,
                e.failing.as_slice(),
            ));
        }
        all
    }

    /// The places that need attention, with what is wrong.
    pub fn failing_items(&self) -> Vec<NoteItem> {
        let v = self.view();
        Self::places_for_notes(&v)
            .into_iter()
            .filter(|p| !p.1)
            .map(|(mut i, _, _, f)| {
                i.detail = f.join("; ");
                i
            })
            .collect()
    }

    pub fn checks_lead(&self) -> String {
        if self.has_no_places() {
            return "There are no places to check.".to_string();
        }
        let v = self.view();
        let all = Self::places_for_notes(&v);
        let failing = all.iter().filter(|p| !p.1).count();
        if failing > 0 {
            format!("{} attention. Open one to see what is wrong and fix it.", plural(failing, "place needs", "places need"))
        } else {
            format!("Every check passes in all {} places.", all.len())
        }
    }

    /// The places that have been changed, and whether each still works.
    pub fn changed_items(&self) -> Vec<NoteItem> {
        let v = self.view();
        Self::places_for_notes(&v)
            .into_iter()
            .filter(|p| p.2)
            .map(|(mut i, ok, _, f)| {
                i.detail = if ok { "Still works".to_string() } else { format!("Needs attention: {}", f.join("; ").to_lowercase()) };
                i
            })
            .collect()
    }

    pub fn changes_lead(&self) -> String {
        if self.has_no_places() {
            return "There are no places to change.".to_string();
        }
        let n = self.changed_items().len();
        if n > 0 {
            format!("{} changed from the city as first laid out.", plural(n, "place", "places"))
        } else {
            "Nothing changed yet. Open a street or a junction to change it.".to_string()
        }
    }

    /// Whether the city works, in one line.
    pub fn status(&self) -> Status {
        if self.has_no_places() {
            // Nothing works, since there is nothing: not a tick.
            return Status { text: NOTHING_TO_SHOW.to_string(), bad: true };
        }
        let v = self.view();
        let bad: Vec<String> = Self::places_for_notes(&v).into_iter().filter(|p| !p.1).map(|p| p.0.name).collect();
        if bad.is_empty() {
            return Status { text: format!("{} places. Every check passes.", v.places), bad: false };
        }
        let more = if bad.len() > 3 { " and more" } else { "" };
        Status {
            text: format!(
                "{} attention: {}{more}.",
                plural(bad.len(), "place needs", "places need"),
                bad.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ),
            bad: true,
        }
    }

    pub fn can_reset(&self) -> bool {
        self.view().edited > 0
    }

    pub fn reset_label(&self) -> &'static str {
        if self.armed.get() { ARMED_RESET } else { RESET }
    }

    pub fn camera(&self) -> Camera {
        self.camera.get()
    }

    pub fn view_box(&self) -> String {
        self.camera.get().view_box()
    }

    /// The place the pointer or the focus is on, which the map and the lists show together.
    pub fn hot(&self) -> Option<String> {
        self.hot.get()
    }

    // ---- commands ----

    pub fn set_hot(&self, hot: Option<String>) {
        if self.hot.get_untracked() != hot {
            self.hot.set(hot);
        }
    }

    pub fn set_region(&self, region: usize) {
        self.core.edit(|m| m.region = region);
    }

    /// The region the city is shown in, by index into the catalogue.
    pub fn region(&self) -> usize {
        self.core.read(|m| m.region)
    }

    /// The city is read again from where it is kept: another tab wrote it, the
    /// page is shown again, or it was started over.
    pub fn reload(&self) {
        let city = self.store.open();
        self.core.edit(|m| m.city = city);
    }

    fn update_camera(&self, f: impl FnOnce(&mut Camera)) {
        self.camera.update(f);
    }

    /// The window the map fills changed size.
    pub fn resize(&self, width: f64, height: f64) {
        self.update_camera(|c| c.resize(width, height));
    }

    // ---- the map ----

    pub fn basemap_state(&self) -> BasemapState {
        self.basemap.get()
    }

    /// Where the city lies on the earth: known once its area's roads are.
    pub fn projection(&self) -> Option<Projection> {
        if !self.store.has_network() {
            return None;
        }
        let area = self.store.area();
        let origin = self.core.view_now().origin_m?;
        Some(Projection::new(area.bounds(), origin))
    }

    /// Listens to what the map says, and tells it the units. Called once, by the page that has a map.
    pub fn attach(&self) {
        let me = self.me.clone();
        self.ports.mapper.listen(Box::new(move |event| {
            if let Some(vm) = me.upgrade() {
                vm.on_map_event(event);
            }
        }));
        self.ports.mapper.set_imperial(matches!(self.core.units(), Units::Feet));
    }

    fn on_map_event(&self, event: MapEvent) {
        match event {
            MapEvent::Ready { bounds } => self.tiles_ready(bounds),
            MapEvent::Failed => self.give_up(MISSING),
            MapEvent::Pick { hot } => {
                if let Some(href) = href_of(&hot) {
                    self.ports.navigator.go(&href);
                }
            }
            MapEvent::Hover { hot } => self.set_hot(hot),
            MapEvent::Moved => self.moved.set(true),
        }
    }

    /// There will be no map, unless the page never had one to wait for (which has said why already).
    fn give_up(&self, words: &'static str) {
        if self.store.has_network() {
            self.basemap.set(BasemapState::Unavailable(words));
        }
    }

    fn tiles_ready(&self, [west, south, east, north]: [f64; 4]) {
        if !self.store.has_network() {
            return;
        }
        let area = self.store.area();
        if !(west..=east).contains(&area.lon) || !(south..=north).contains(&area.lat) {
            return self.give_up(OUTSIDE);
        }
        self.basemap.set(BasemapState::Ready);
        self.sync_places();
        self.fit();
    }

    /// Sends the places to the map, once there is a map to show them on.
    pub fn sync_places(&self) {
        let Some(projection) = self.projection() else { return };
        let area = self.store.area();
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        self.ports.mapper.set_places(&style::layers(area.lat), &overlay::places(&self.core.view_now(), &projection));
    }

    /// Sends the place the pointer or the focus is on to the map.
    pub fn sync_highlight(&self) {
        self.ports.mapper.highlight(self.hot.get_untracked().as_deref());
    }

    /// Fits the places in what the panels leave of the map, and gives the view back to the page.
    pub fn fit(&self) {
        self.moved.set(false);
        let Some(projection) = self.projection() else { return };
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        let Some(bounds) = overlay::bounds(&self.core.view_now(), &projection) else { return };
        let i = self.camera.get_untracked().insets;
        self.ports.mapper.fit(bounds, [i.top, i.right, i.bottom, i.left]);
    }

    /// The colours the places are drawn in, and what each says.
    pub fn status_key(&self) -> Vec<(&'static str, &'static str)> {
        vec![("ok", "Works"), ("changed", "Changed"), ("bad", "Needs attention")]
    }

    /// What floats over the map, in pixels from each edge: the whole city is fitted in what is left, until
    /// the person moves the map themselves.
    pub fn set_insets(&self, insets: Insets) {
        self.update_camera(|c| c.set_insets(insets));
        if !self.moved.get() {
            self.fit();
        }
    }

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
        self.ports.mapper.set_imperial(matches!(units, Units::Feet));
    }

    pub fn zoom_in(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(ZOOM_STEP);
    }

    pub fn zoom_out(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(1.0 / ZOOM_STEP);
    }

    fn pan(&self, dx: f64, dy: f64) {
        self.moved.set(true);
        self.ports.mapper.pan_by(dx, dy);
    }

    /// The whole city fits the window again: the hero's camera, and the map.
    pub fn fit_camera(&self) {
        self.update_camera(|c| c.fit());
        self.fit();
    }

    /// What a key does to the map; says whether the key was one of its own.
    pub fn press_key(&self, key: &str) -> bool {
        match key {
            "ArrowLeft" => self.pan(-PAN_PX, 0.0),
            "ArrowRight" => self.pan(PAN_PX, 0.0),
            "ArrowUp" => self.pan(0.0, -PAN_PX),
            "ArrowDown" => self.pan(0.0, PAN_PX),
            "+" | "=" => self.zoom_in(),
            "-" | "_" => self.zoom_out(),
            "0" => self.fit_camera(),
            _ => return false,
        }
        true
    }

    /// "Start over" undoes every change in the whole city and cannot itself be
    /// undone, so it asks to be pressed twice.
    pub fn press_reset(&self) -> ResetOutcome {
        if !self.can_reset() {
            return ResetOutcome::Ignored;
        }
        if !self.armed.get_untracked() {
            self.armed.set(true);
            self.ports.announcer.say(ARMED_SAID);
            return ResetOutcome::Armed;
        }
        self.armed.set(false);
        self.store.reset();
        self.reload();
        self.ports.announcer.say(DONE_SAID);
        ResetOutcome::Done
    }

    /// Lets go of a first press of "start over".
    pub fn disarm(&self) {
        if self.armed.get_untracked() {
            self.armed.set(false);
        }
    }
}

impl crate::shell::Target for MapVm {
    fn set_units(&self, units: Units) {
        MapVm::set_units(self, units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.set_region(region);
        true
    }

    fn region_id(&self) -> String {
        crate::shared::catalogue::REGIONS[self.region()].id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::camera::Insets;
    use crate::place::area::Area;
    use crate::shared::ports::{FakeMapper, MapCall, MapEvent, MemoryStorage, Ports, RecordingAnnouncer, RecordingNavigator, test_ports};
    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};

    fn vm() -> (Rc<MapVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (ports, said, storage) = test_ports();
        (MapVm::on_sample(ports), said, storage)
    }

    /// Another page writes a street of the city into the shared storage.
    fn nudge_street(storage: &Rc<MemoryStorage>, street: u32, mm: i32) {
        let store = CityStore::sample(storage.clone());
        assert!(store.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            assert!(e.set_width(u, e.view().segments[0].width_mm + mm), "{mm}");
            c.keep_street(street, e.snapshot())
        }));
    }

    fn names(vm: &MapVm) -> Vec<String> {
        vm.results().into_iter().map(|r| r.name).collect()
    }

    #[test]
    fn nothing_is_found_until_something_is_searched_for_and_the_places_list_is_whole() {
        let (vm, ..) = vm();
        assert_eq!(vm.search_text(), "");
        assert!(vm.results().is_empty());
        assert_eq!(vm.search_note(), None);
        vm.set_search("avenue");
        assert_eq!(vm.junction_rows().len() + vm.street_rows().len(), vm.places(), "the places list is not narrowed");
    }

    #[test]
    fn a_search_finds_places_by_name_and_by_the_small_print() {
        let (vm, ..) = vm();
        vm.set_search("avenue");
        assert!(!vm.results().is_empty() && names(&vm).iter().all(|n| n.to_lowercase().contains("avenue")));
        vm.set_search("signal");
        let found = vm.results();
        assert!(!found.is_empty() && found.iter().all(|r| r.sub.to_lowercase().contains("traffic signal")));
    }

    #[test]
    fn a_search_ignores_case_and_spaces_looks_for_the_phrase_first_and_then_for_each_word() {
        let (vm, ..) = vm();
        vm.set_search("  JUNCTION   4 ");
        let found = vm.results();
        assert!(found.iter().any(|r| r.name == "Junction 4"));
        assert!(found.iter().all(|r| format!("{} {}", r.name, r.sub).contains("Junction 4")), "not every junction of four streets");
        vm.set_search("avenue junction 5");
        let found = vm.results();
        assert!(!found.is_empty() && found.iter().all(|r| r.name.to_lowercase().contains("avenue") && r.sub.contains("Junction 5")));
    }

    #[test]
    fn the_note_counts_the_matches_and_says_what_to_do_when_there_are_none() {
        let (vm, ..) = vm();
        vm.set_search("Junction 1 stop");
        assert_eq!(vm.search_note().as_deref(), Some("1 place matches"));
        vm.set_search("zzz");
        assert_eq!(vm.search_note(), Some(format!("No places match \u{201c}zzz\u{201d}. Clear the search to see all {}.", vm.places())));
        vm.set_search("sample");
        assert!(vm.search_note().unwrap().ends_with(&format!("showing the first {}", MapVm::SHOWN)));
    }

    #[test]
    fn the_arrow_keys_move_down_the_results_and_stop_at_the_ends() {
        let (vm, ..) = vm();
        vm.set_search("junction");
        assert_eq!(vm.active_result(), None);
        vm.move_active(-1);
        assert_eq!(vm.active_result(), Some(0));
        vm.move_active(1);
        vm.move_active(1);
        assert_eq!(vm.active_result(), Some(2));
        for _ in 0..20 {
            vm.move_active(1);
        }
        assert_eq!(vm.active_result(), Some(MapVm::SHOWN - 1));
        vm.set_search("junction 4");
        assert_eq!(vm.active_result(), None, "a new search starts again");
        vm.set_search("zzz");
        vm.move_active(1);
        assert_eq!(vm.active_result(), None);
    }

    #[test]
    fn enter_opens_the_result_the_arrows_are_on_or_else_the_first_and_nothing_when_there_is_none() {
        let (vm, ..) = vm();
        vm.set_search("junction");
        assert_eq!(vm.chosen_href(), vm.results().first().map(|r| r.href.clone()));
        vm.move_active(1);
        vm.move_active(1);
        assert_eq!(vm.chosen_href(), vm.results().get(1).map(|r| r.href.clone()));
        vm.set_search("avenue");
        assert!(vm.chosen_href().unwrap().starts_with("street.html?street="));
        vm.set_search("zzz");
        assert_eq!(vm.chosen_href(), None);
    }

    #[test]
    fn clearing_the_search_finds_nothing_and_says_nothing() {
        let (vm, ..) = vm();
        vm.set_search("avenue");
        vm.set_search("");
        assert_eq!(vm.search_note(), None);
        assert_eq!(vm.chosen_href(), None);
    }

    #[test]
    fn what_floats_over_the_map_makes_the_whole_city_fit_in_the_open_part() {
        let (vm, ..) = vm();
        vm.resize(1000.0, 600.0);
        let open = vm.camera().fit_k;
        vm.set_insets(Insets { left: 376.0, top: 88.0, right: 376.0, bottom: 72.0 });
        assert!(vm.camera().fit_k < open);
        assert_eq!(vm.camera().k, vm.camera().fit_k);
        vm.zoom_in();
        vm.fit_camera();
        assert_eq!(vm.camera().k, vm.camera().fit_k, "whole city fits the open part again");
    }

    #[test]
    fn plural_counts_with_the_right_word() {
        assert_eq!(plural(1, "street", "streets"), "1 street");
        assert_eq!(plural(2, "place needs", "places need"), "2 places need");
        assert_eq!(plural(0, "place", "places"), "0 places");
    }

    #[test]
    fn a_street_runs_between_the_places_after_the_dot_in_its_name() {
        assert_eq!(street_ends("Avenue \u{b7} Junction 1 to Junction 2"), "Junction 1 to Junction 2");
        assert_eq!(street_ends("Plain"), "");
    }

    #[test]
    fn the_title_and_counts_describe_the_city() {
        let (vm, ..) = vm();
        let owner = Owner::new();
        owner.set();
        let v = vm.view();
        assert_eq!(vm.title(), v.name);
        assert_eq!(vm.counts(), format!("{}, {}", plural(v.nodes.iter().filter(|n| n.junction).count(), "junction", "junctions"), streets(v.edges.len())));
        assert_eq!((vm.places(), vm.changes()), (v.places, 0));
    }

    #[test]
    fn the_junctions_are_listed_in_the_order_of_their_numbers_with_a_link_to_each() {
        let (vm, ..) = vm();
        let rows = vm.junction_rows();
        let view = vm.view();
        let numbers: Vec<u32> = rows.iter().map(|r| view.nodes.iter().find(|n| n.name == r.name).unwrap().number).collect();
        let mut sorted = numbers.clone();
        sorted.sort_unstable();
        assert_eq!(numbers, sorted);
        assert!(rows[0].href.starts_with("intersection.html?junction=") && rows[0].hot.starts_with("j-"));
        assert!(rows[0].sub.contains(" street"));
        assert_eq!(rows[0].tag, None);
    }

    #[test]
    fn the_streets_are_listed_with_where_they_run_and_how_wide_in_the_units_shown() {
        let (vm, ..) = vm();
        let rows = vm.street_rows();
        assert_eq!(rows.len(), vm.view().edges.len());
        let e = &vm.view().edges[0];
        assert_eq!(rows[0].href, format!("street.html?street={}", e.uid));
        assert_eq!(rows[0].hot, format!("s-{}", e.uid));
        assert_eq!(rows[0].sub, format!("{} \u{b7} {}", street_ends(&e.name), Units::Metres.length(e.row_mm)));
        vm.set_units(Units::Feet);
        assert!(vm.street_rows()[0].sub.ends_with(&Units::Feet.length(e.row_mm)));
    }

    #[test]
    fn a_place_tells_a_screen_reader_what_it_is_and_what_it_opens() {
        let (vm, ..) = vm();
        let (n, e) = (vm.view().nodes.iter().find(|n| n.junction).unwrap().uid, vm.view().edges[0].uid);
        let j = vm.junction_label(n);
        assert!(j.ends_with(" Opens the junction plan.") && j.contains("street"), "{j}");
        let s = vm.street_label(e);
        assert!(s.ends_with(" wide. Opens the street cross-section."), "{s}");
        assert_eq!(vm.junction_label(9_999), "");
    }

    #[test]
    fn a_street_changed_by_another_page_shows_as_changed_after_the_map_reads_again() {
        let (vm, _, storage) = vm();
        let e = vm.view().edges[0].uid;
        assert_eq!(vm.changes(), 0);
        nudge_street(&storage, e, -100);
        assert_eq!(vm.changes(), 0, "not until it is read again");
        vm.reload();
        assert_eq!(vm.changes(), 1);
        assert_eq!(vm.street_rows()[0].tag, Some(StateTag { text: "Changed", bad: false }));
        assert!(vm.street_label(e).contains(" Changed."));
        let items = vm.changed_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].detail, "Still works");
        assert_eq!(vm.changes_lead(), "1 place changed from the city as first laid out.");
    }

    #[test]
    fn a_street_that_no_longer_works_is_flagged_everywhere_it_is_listed() {
        let (vm, _, storage) = vm();
        let e = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
        nudge_street(&storage, e, 1_000);
        vm.reload();
        assert!(!vm.status().text.starts_with(&format!("{} places", vm.places())));
        let st = vm.status();
        assert!(st.bad && st.text.starts_with("1 place needs attention: "), "{}", st.text);
        let row = vm.street_rows().into_iter().find(|r| r.hot == format!("s-{e}")).unwrap();
        assert_eq!(row.tag, Some(StateTag { text: "Needs attention", bad: true }));
        assert!(vm.street_label(e).contains(" Needs attention: "));
        assert_eq!(vm.failing_items().len(), 1);
        assert!(vm.checks_lead().starts_with("1 place needs attention. Open one"));
        assert!(vm.changed_items()[0].detail.starts_with("Needs attention: "));
    }

    #[test]
    fn a_city_that_works_says_so_for_every_place() {
        let (vm, ..) = vm();
        let st = vm.status();
        assert_eq!(st, Status { text: format!("{} places. Every check passes.", vm.places()), bad: false });
        assert_eq!(vm.checks_lead(), format!("Every check passes in all {} places.", vm.places()));
        assert!(vm.failing_items().is_empty());
        assert_eq!(vm.changes_lead(), "Nothing changed yet. Open a street or a junction to change it.");
    }

    #[test]
    fn the_status_names_three_places_and_says_there_are_more() {
        let (vm, _, storage) = vm();
        let streets: Vec<u32> = vm.view().edges.iter().filter(|e| !e.freeway).map(|e| e.uid).take(4).collect();
        for s in streets {
            nudge_street(&storage, s, 1_000);
        }
        vm.reload();
        let st = vm.status();
        assert!(st.text.starts_with("4 places need attention: ") && st.text.ends_with(" and more."), "{}", st.text);
        let named = vm.failing_items();
        for item in &named[..3] {
            assert!(st.text.contains(&item.name), "{} in {}", item.name, st.text);
        }
        assert!(!st.text.contains(&named[3].name) || named[3].name == named[0].name);
    }

    #[test]
    fn start_over_asks_to_be_pressed_twice_and_then_puts_the_city_back() {
        let (vm, said, storage) = vm();
        assert!(!vm.can_reset());
        assert_eq!(vm.press_reset(), ResetOutcome::Ignored);
        nudge_street(&storage, vm.view().edges[0].uid, -100);
        vm.reload();
        assert!(vm.can_reset());
        assert_eq!(vm.reset_label(), "Start over");
        assert_eq!(vm.press_reset(), ResetOutcome::Armed);
        assert_eq!(vm.reset_label(), "Press again to start over");
        assert_eq!(said.take(), vec![ARMED_SAID]);
        assert_eq!(vm.changes(), 1, "nothing is undone yet");
        assert_eq!(vm.press_reset(), ResetOutcome::Done);
        assert_eq!((vm.changes(), vm.reset_label()), (0, "Start over"));
        assert_eq!(said.take(), vec![DONE_SAID]);
        assert_eq!(CityStore::sample(storage.clone()).open().view(0).edited, 0, "and it is kept");
    }

    #[test]
    fn letting_go_of_the_first_press_puts_the_button_back() {
        let (vm, _, storage) = vm();
        nudge_street(&storage, vm.view().edges[0].uid, -100);
        vm.reload();
        vm.press_reset();
        vm.disarm();
        assert_eq!(vm.reset_label(), "Start over");
        assert_eq!(vm.press_reset(), ResetOutcome::Armed, "it must be asked again");
        assert_eq!(vm.changes(), 1);
    }

    #[test]
    fn the_region_is_read_from_storage_and_can_be_changed() {
        let (ports, ..) = test_ports();
        ports.storage.remember(crate::city::store::REGION_KEY, "united-kingdom");
        let vm = MapVm::on_sample(ports);
        assert_eq!(vm.region(), 3);
        vm.set_region(0);
        assert_eq!(vm.region(), 0);
    }

    #[test]
    fn the_hot_place_is_set_and_cleared_by_the_view() {
        let (vm, ..) = vm();
        assert_eq!(vm.hot(), None);
        vm.set_hot(Some("s-3".into()));
        assert_eq!(vm.hot().as_deref(), Some("s-3"));
        vm.set_hot(None);
        assert_eq!(vm.hot(), None);
    }

    /// A city of one street that bends, kept for an area that the page opens.
    fn area_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        let area = Area::new("Bendville", 45.5, -73.6);
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0 };
        let network = Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        };
        assert!(CityStore::for_area(storage.clone(), area.clone()).keep_network(&network));
        assert!(CityStore::choose(&*storage, &area));
        let (mapper, navigator) = (Rc::new(FakeMapper::default()), Rc::new(RecordingNavigator::default()));
        let ports = Ports { mapper: mapper.clone(), navigator: navigator.clone(), ..ports };
        let vm = MapVm::new(ports);
        vm.attach();
        mapper.take(); // the scale's units, said when it attached
        (vm, mapper, navigator, storage)
    }

    /// Tiles that cover the area `area_vm` opens.
    const COVERING: MapEvent = MapEvent::Ready { bounds: [-74.0, 45.0, -73.0, 46.0] };

    /// The map page on the area chosen (here the default one), whose roads were never kept.
    fn roadless_map_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        let mapper = Rc::new(FakeMapper::default());
        let vm = MapVm::new(Ports { mapper: mapper.clone(), ..ports });
        vm.attach();
        mapper.take();
        (vm, mapper, storage)
    }

    #[test]
    fn the_map_page_of_an_area_whose_roads_were_not_had_shows_no_places_and_names_the_area() {
        let (vm, mapper, _) = roadless_map_vm();
        let owner = Owner::new();
        owner.set();
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(NO_ROADS));
        assert_eq!(vm.title(), crate::place::area::default_area().name, "the area that could not be loaded");
        assert!(vm.junction_rows().is_empty() && vm.street_rows().is_empty());
        assert_eq!((vm.places(), vm.changes()), (0, 0));
        assert_eq!(vm.counts(), "0 junctions, 0 streets");
        assert_eq!(vm.status(), Status { text: NOTHING_TO_SHOW.to_string(), bad: true });
        assert!(vm.failing_items().is_empty() && vm.changed_items().is_empty());
        assert_eq!(vm.checks_lead(), "There are no places to check.");
        assert_eq!(vm.changes_lead(), "There are no places to change.");
        assert!(!vm.can_reset());
        assert_eq!(vm.press_reset(), ResetOutcome::Ignored);
        vm.set_search("avenue");
        assert!(vm.results().is_empty());
        assert_eq!(vm.search_note().as_deref(), Some("There are no places to search."));
        assert!(vm.projection().is_none());
        // the camera the page keeps for the panels has a box round nothing, not an inside-out one
        vm.resize(1000.0, 600.0);
        vm.set_insets(Insets { left: 300.0, ..Insets::default() });
        assert!(vm.camera().fit_k.is_finite() && vm.camera().fit_k > 0.0);
        mapper.say(COVERING);
        mapper.say(MapEvent::Failed);
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(NO_ROADS), "the reason stays the roads");
        assert!(mapper.take().is_empty(), "nothing is drawn");
    }

    #[test]
    fn the_map_page_reads_nothing_back_from_another_city_s_keys_when_the_city_is_read_again() {
        let (vm, _, storage) = roadless_map_vm();
        nudge_street(&storage, 1, -100); // under the keys of no area
        vm.reload();
        assert_eq!((vm.places(), vm.changes()), (0, 0));
        assert_eq!(vm.title(), crate::place::area::default_area().name);
    }

    #[test]
    fn the_map_page_of_an_area_whose_roads_were_had_shows_them() {
        let (vm, ..) = area_vm();
        assert_eq!((vm.title().as_str(), vm.street_rows().len()), ("Bendville", 1));
    }

    #[test]
    fn the_map_waits_for_the_tiles_and_then_shows_the_places_and_fits_them() {
        let (vm, mapper, ..) = area_vm();
        assert_eq!(vm.basemap_state(), BasemapState::Waiting);
        assert!(mapper.take().is_empty(), "nothing is shown before the tiles are there");
        mapper.say(COVERING);
        assert_eq!(vm.basemap_state(), BasemapState::Ready);
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Places { layers, data } if layers.contains("places-street") && data.contains("\"s-1\"")), "{calls:?}");
        assert!(matches!(&calls[1], MapCall::Fit { bounds, insets } if bounds[0] < bounds[2] && bounds[1] < bounds[3] && *insets == [0.0; 4]), "{calls:?}");
    }

    #[test]
    fn tiles_that_do_not_cover_the_area_say_so_and_show_nothing() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Ready { bounds: [10.0, 50.0, 11.0, 51.0] });
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(OUTSIDE));
        assert!(mapper.take().is_empty());
        vm.sync_places();
        assert!(mapper.take().is_empty(), "still nothing, though asked");
    }

    #[test]
    fn tiles_that_could_not_be_had_say_so() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Failed);
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(MISSING));
    }

    #[test]
    fn pressing_a_place_opens_it_and_the_pointer_over_one_makes_it_hot() {
        let (vm, mapper, navigator, _) = area_vm();
        mapper.say(MapEvent::Pick { hot: "s-1".into() });
        mapper.say(MapEvent::Pick { hot: "j-3".into() });
        mapper.say(MapEvent::Pick { hot: "nonsense".into() });
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string(), "intersection.html?junction=3".to_string()]);
        mapper.say(MapEvent::Hover { hot: Some("s-1".into()) });
        assert_eq!(vm.hot().as_deref(), Some("s-1"));
        mapper.say(MapEvent::Hover { hot: None });
        assert_eq!(vm.hot(), None);
    }

    #[test]
    fn the_hot_place_is_sent_to_the_map() {
        let (vm, mapper, ..) = area_vm();
        vm.set_hot(Some("s-1".into()));
        vm.sync_highlight();
        vm.set_hot(None);
        vm.sync_highlight();
        assert_eq!(mapper.take(), vec![MapCall::Highlight(Some("s-1".into())), MapCall::Highlight(None)]);
    }

    #[test]
    fn the_buttons_and_keys_move_the_map_through_the_port() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.zoom_in();
        vm.zoom_out();
        assert!(vm.press_key("+") && vm.press_key("=") && vm.press_key("-") && vm.press_key("_"));
        assert!(vm.press_key("ArrowLeft") && vm.press_key("ArrowRight") && vm.press_key("ArrowUp") && vm.press_key("ArrowDown"));
        assert!(!vm.press_key("a"));
        assert!(vm.press_key("0"));
        let calls = mapper.take();
        let (zin, zout) = (1.4, 1.0 / 1.4);
        assert_eq!(
            calls[..10],
            [
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zin),
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zout),
                MapCall::Pan(-80.0, 0.0),
                MapCall::Pan(80.0, 0.0),
                MapCall::Pan(0.0, -80.0),
                MapCall::Pan(0.0, 80.0),
            ]
        );
        assert!(matches!(calls[10], MapCall::Fit { .. }), "{calls:?}");
    }

    #[test]
    fn the_places_are_fitted_in_what_floats_over_the_map_until_the_person_moves_it() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.set_insets(Insets { left: 100.0, top: 50.0, right: 20.0, bottom: 10.0 });
        assert!(matches!(mapper.take()[..], [MapCall::Fit { insets, .. }] if insets == [50.0, 20.0, 10.0, 100.0]), "top, right, bottom, left");
        mapper.say(MapEvent::Moved);
        vm.set_insets(Insets { left: 200.0, ..Insets::default() });
        assert!(mapper.take().is_empty(), "the person's view is kept");
        vm.fit_camera();
        assert!(matches!(mapper.take()[..], [MapCall::Fit { insets, .. }] if insets == [0.0, 0.0, 0.0, 200.0]));
        vm.set_insets(Insets::default());
        assert_eq!(mapper.take().len(), 1, "fitting gave the view back to the page");
    }

    #[test]
    fn the_units_are_told_to_the_scale() {
        let (vm, mapper, ..) = area_vm();
        vm.set_units(Units::Feet);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(true)]);
        vm.set_units(Units::Metres);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(false)]);
    }

    #[test]
    fn the_key_says_what_the_colours_of_the_places_mean() {
        let (vm, ..) = vm();
        assert_eq!(vm.status_key(), vec![("ok", "Works"), ("changed", "Changed"), ("bad", "Needs attention")]);
    }

    #[test]
    fn the_hero_camera_follows_the_window_and_the_view_box_follows_it() {
        let (vm, ..) = vm();
        let o = Owner::new();
        o.set();
        vm.resize(1000.0, 600.0);
        let fit = vm.camera();
        assert_eq!((fit.width, fit.height), (1000.0, 600.0));
        let before = vm.view_box();
        vm.resize(500.0, 600.0);
        assert_ne!(vm.view_box(), before);
    }
}
