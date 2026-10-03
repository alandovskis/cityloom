//! The city map's view-model: how the city reads as places to open, what needs
//! attention and what has changed, where the camera is, and what pressing
//! "start over" means. The view binds to its properties and sends it commands.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::KINDS;
use crate::city::model::{City, CityView, EdgeView, NodeView};
use crate::shared::units::Units;
use crate::vm::camera::{Camera, ScaleBar, World};
use crate::city::store::CityStore;
use crate::shared::core::{Core, Presents};
use crate::vm::map_gestures::{MapGestures, Moved};
use crate::shared::ports::Ports;

pub const ARMED_RESET: &str = "Press again to start over";
pub const RESET: &str = "Start over";
const ARMED_SAID: &str = "This puts every street and junction back as first laid out. Press again to confirm.";
const DONE_SAID: &str = "The city is back as it was first laid out.";

/// How far a zoom button, or a key, zooms at once.
const ZOOM_STEP: f64 = 1.4;
/// How far an arrow key pans, in pixels.
const PAN_PX: f64 = 80.0;

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

/// The number of a junction from its name: `Junction 12` is 12.
pub fn junction_number(name: &str) -> u32 {
    name.trim_start_matches(|c: char| !c.is_ascii_digit()).parse().unwrap_or(0)
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
    format!(
        "{}, {}, {}.{} Opens the junction plan.",
        n.name,
        n.control.unwrap_or("").to_lowercase(),
        streets(n.arms),
        state_words(n.ok, &n.failing, n.edited)
    )
}

/// How a street on the map is told to a screen reader.
pub fn street_label(e: &EdgeView, units: Units) -> String {
    format!("{}, {}, {} wide.{} Opens the street cross-section.", e.kind, street_ends(&e.name), units.length(e.row_mm), state_words(e.ok, &e.failing, e.edited))
}

pub fn street_href(uid: u32) -> String {
    format!("index.html?street={uid}")
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
    core: Core<CityModel>,
    store: CityStore,
    ports: Ports,
    camera: ArcRwSignal<Camera>,
    gestures: RefCell<MapGestures>,
    panning: ArcRwSignal<bool>,
    hot: ArcRwSignal<Option<String>>,
    armed: ArcRwSignal<bool>,
}

impl MapVm {
    pub fn new(ports: Ports) -> Rc<MapVm> {
        let store = CityStore::new(ports.storage.clone());
        let model = CityModel { city: store.open(), region: store.region() };
        let core = Core::new(model);
        let world = World::round(core.view_now().bounds_mm);
        Rc::new(MapVm {
            core,
            store,
            ports,
            camera: ArcRwSignal::new(Camera::new(world, 800.0, 520.0)),
            gestures: RefCell::new(MapGestures::default()),
            panning: ArcRwSignal::new(false),
            hot: ArcRwSignal::new(None),
            armed: ArcRwSignal::new(false),
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
        j.sort_by_key(|n| junction_number(&n.name));
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

    pub fn changes(&self) -> usize {
        self.view().edited
    }

    fn tag(ok: bool, edited: bool) -> Option<StateTag> {
        if !ok {
            Some(StateTag { text: "Needs attention", bad: true })
        } else {
            edited.then_some(StateTag { text: "Changed", bad: false })
        }
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
            all.push((NoteItem { href: street_href(e.uid), name: format!("{}, {}", e.kind, street_ends(&e.name)), detail: String::new() }, e.ok, e.edited, e.failing.as_slice()));
        }
        all
    }

    /// The places that need attention, with what is wrong.
    pub fn failing_items(&self) -> Vec<NoteItem> {
        let v = self.view();
        Self::places_for_notes(&v).into_iter().filter(|p| !p.1).map(|(mut i, _, _, f)| { i.detail = f.join("; "); i }).collect()
    }

    pub fn checks_lead(&self) -> String {
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
        let n = self.changed_items().len();
        if n > 0 {
            format!("{} changed from the city as first laid out.", plural(n, "place", "places"))
        } else {
            "Nothing changed yet. Open a street or a junction to change it.".to_string()
        }
    }

    /// Whether the city works, in one line.
    pub fn status(&self) -> Status {
        let v = self.view();
        let bad: Vec<String> = Self::places_for_notes(&v).into_iter().filter(|p| !p.1).map(|p| p.0.name).collect();
        if bad.is_empty() {
            return Status { text: format!("{} places. Every check passes.", v.places), bad: false };
        }
        let more = if bad.len() > 3 { " and more" } else { "" };
        Status { text: format!("{} attention: {}{more}.", plural(bad.len(), "place needs", "places need"), bad.iter().take(3).cloned().collect::<Vec<_>>().join(", ")), bad: true }
    }

    /// The kinds of piece the streets are made of, in the order the catalogue lists them.
    pub fn legend(&self) -> Vec<(&'static str, &'static str)> {
        let v = self.view();
        KINDS.iter().filter(|k| v.edges.iter().any(|e| e.pieces.iter().any(|p| p.kind == k.id))).map(|k| (k.id, k.name)).collect()
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

    pub fn scale_bar(&self) -> ScaleBar {
        self.camera.get().scale_bar(self.units())
    }

    pub fn panning(&self) -> bool {
        self.panning.get()
    }

    /// The place the pointer or the focus is on, which the map and the lists show together.
    pub fn hot(&self) -> Option<String> {
        self.hot.get()
    }

    // ---- commands ----

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
    }

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

    pub fn zoom_in(&self) {
        self.update_camera(|c| c.zoom_by(ZOOM_STEP));
    }

    pub fn zoom_out(&self) {
        self.update_camera(|c| c.zoom_by(1.0 / ZOOM_STEP));
    }

    pub fn fit_camera(&self) {
        self.update_camera(|c| c.fit());
    }

    /// A turn of the wheel at a point (pixels from the window's middle); a pinch on
    /// a trackpad comes as a wheel with Ctrl held, and is finer.
    pub fn wheel(&self, delta_y: f64, ctrl: bool, sx: f64, sy: f64) {
        self.update_camera(|c| c.zoom_at(c.k * (-delta_y * if ctrl { 0.01 } else { 0.0015 }).exp(), sx, sy));
    }

    /// What a key does to the map; says whether the key was one of its own.
    pub fn press_key(&self, key: &str) -> bool {
        match key {
            "ArrowLeft" => self.update_camera(|c| c.pan_by(-PAN_PX, 0.0)),
            "ArrowRight" => self.update_camera(|c| c.pan_by(PAN_PX, 0.0)),
            "ArrowUp" => self.update_camera(|c| c.pan_by(0.0, -PAN_PX)),
            "ArrowDown" => self.update_camera(|c| c.pan_by(0.0, PAN_PX)),
            "+" | "=" => self.zoom_in(),
            "-" | "_" => self.zoom_out(),
            "0" => self.fit_camera(),
            _ => return false,
        }
        true
    }

    pub fn pointer_down(&self, id: i32, x: f64, y: f64) {
        let camera = self.camera.get_untracked();
        self.gestures.borrow_mut().down(&camera, id, x, y);
    }

    /// A pointer moved; `origin` is the middle of the window in page pixels.
    /// Says whether a drag has just begun, so that the view can take hold of the pointer.
    pub fn pointer_move(&self, id: i32, x: f64, y: f64, origin: (f64, f64)) -> bool {
        let mut camera = self.camera.get_untracked();
        let moved = self.gestures.borrow_mut().moved(&mut camera, id, x, y, origin);
        if moved != Moved::Nothing {
            self.camera.set(camera);
        }
        let panning = self.gestures.borrow().panning();
        if self.panning.get_untracked() != panning {
            self.panning.set(panning);
        }
        moved == Moved::StartedPanning
    }

    pub fn pointer_up(&self, id: i32) {
        self.gestures.borrow_mut().up(id);
        if self.panning.get_untracked() {
            self.panning.set(false);
        }
    }

    /// Whether a click is the end of a drag, and so is not to open a place.
    pub fn swallow_click(&self) -> bool {
        self.gestures.borrow_mut().swallow_click()
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

impl crate::vm::shell::Target for MapVm {
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
    use crate::shared::ports::{MemoryStorage, RecordingAnnouncer, test_ports};

    fn vm() -> (Rc<MapVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (ports, said, storage) = test_ports();
        (MapVm::new(ports), said, storage)
    }

    /// Another page writes a street of the city into the shared storage.
    fn nudge_street(storage: &Rc<MemoryStorage>, street: u32, mm: i32) {
        let store = CityStore::new(storage.clone());
        assert!(store.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            assert!(e.set_width(u, e.view().segments[0].width_mm + mm), "{mm}");
            c.keep_street(street, e.snapshot())
        }));
    }

    #[test]
    fn plural_counts_with_the_right_word() {
        assert_eq!(plural(1, "street", "streets"), "1 street");
        assert_eq!(plural(2, "place needs", "places need"), "2 places need");
        assert_eq!(plural(0, "place", "places"), "0 places");
    }

    #[test]
    fn a_junction_is_numbered_by_the_digits_in_its_name() {
        assert_eq!(junction_number("Junction 12"), 12);
        assert_eq!(junction_number("Junction 4"), 4);
        assert_eq!(junction_number("Edge"), 0);
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
        let numbers: Vec<u32> = rows.iter().map(|r| junction_number(&r.name)).collect();
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
        assert_eq!(rows[0].href, format!("index.html?street={}", e.uid));
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
    fn the_legend_lists_the_kinds_the_streets_are_made_of_in_catalogue_order() {
        let (vm, ..) = vm();
        let legend = vm.legend();
        assert!(!legend.is_empty());
        let positions: Vec<usize> = legend.iter().map(|(id, _)| KINDS.iter().position(|k| k.id == *id).unwrap()).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
        assert!(legend.iter().any(|(id, name)| *id == "sidewalk" && *name == "Sidewalk"));
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
        assert_eq!(CityStore::new(storage.clone()).open().view(0).edited, 0, "and it is kept");
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
        let vm = MapVm::new(ports);
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

    #[test]
    fn the_camera_follows_the_window_and_the_buttons_and_keys() {
        let (vm, ..) = vm();
        vm.resize(1000.0, 600.0);
        let fit = vm.camera();
        assert_eq!((fit.width, fit.height), (1000.0, 600.0));
        vm.zoom_in();
        assert!((vm.camera().k - fit.k * 1.4).abs() < 1e-9);
        vm.zoom_out();
        assert!((vm.camera().k - fit.k).abs() < 1e-9);
        assert!(vm.press_key("+") && vm.camera().k > fit.k);
        assert!(vm.press_key("0") && vm.camera().k == vm.camera().fit_k && !vm.camera().moved);
        let cx = vm.camera().cx;
        assert!(vm.press_key("ArrowRight") && vm.camera().cx > cx);
        assert!(!vm.press_key("a"));
        vm.fit_camera();
        assert_eq!(vm.camera().cx, cx);
    }

    #[test]
    fn the_wheel_zooms_about_the_pointer_and_finer_with_control() {
        let (vm, ..) = vm();
        let k0 = vm.camera().k;
        vm.wheel(-100.0, false, 0.0, 0.0);
        let plain = vm.camera().k / k0;
        vm.fit_camera();
        vm.wheel(-100.0, true, 0.0, 0.0);
        let ctrl = vm.camera().k / k0;
        assert!((plain - (0.15f64).exp()).abs() < 1e-9 && (ctrl - 1.0f64.exp()).abs() < 1e-9);
        vm.wheel(10_000.0, false, 0.0, 0.0);
        assert!(vm.camera().k < k0);
    }

    #[test]
    fn the_view_box_and_scale_bar_follow_the_camera_and_the_units() {
        let (vm, ..) = vm();
        let o = Owner::new();
        o.set();
        let before = vm.view_box();
        vm.zoom_in();
        assert_ne!(vm.view_box(), before);
        let m = vm.scale_bar();
        assert_eq!(m.unit, "m");
        vm.set_units(Units::Feet);
        assert_eq!(vm.scale_bar().unit, "ft");
    }

    #[test]
    fn a_drag_pans_the_map_and_the_click_that_ends_it_is_swallowed() {
        let (vm, ..) = vm();
        let c0 = vm.camera();
        vm.pointer_down(1, 100.0, 100.0);
        assert!(!vm.pointer_move(1, 102.0, 100.0, (400.0, 260.0)), "too short to be a drag");
        assert!(vm.pointer_move(1, 160.0, 100.0, (400.0, 260.0)), "a drag begins");
        assert!(vm.panning());
        assert!(!vm.pointer_move(1, 170.0, 100.0, (400.0, 260.0)));
        assert!(vm.camera().cx < c0.cx);
        vm.pointer_up(1);
        assert!(!vm.panning());
        assert!(vm.swallow_click());
        assert!(!vm.swallow_click());
    }
}
