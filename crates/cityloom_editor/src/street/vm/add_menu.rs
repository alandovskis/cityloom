//! The street page's Add a piece menu as the view-model has it: eight topics, each holding everything of its subject
//! whatever kind of thing it is (a piece, a green element, a transit stop, a traffic calming element), the topic the
//! menu opens on, which items cannot be added to the street as it is and why, and the search through every topic.
//! Pure: it reads the street's view; the view-model adds and announces.

use crate::shared::catalogue::{KINDS, group_key, kind_index, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::Said;
use crate::shared::search::fold;
use crate::street::model::{CalmingKind, CalmingKindExt, GreenKind, GreenKindExt, StopKind, StopKindExt, View};
use crate::street::vm::level::Level;

/// A topic of the menu: what a person looks for, not how the street keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    Walking,
    Greenery,
    Cycling,
    Transit,
    Roadway,
    Calming,
    Furniture,
    Utilities,
}

impl Topic {
    /// Every topic, in the menu's order.
    pub const ALL: [Topic; 8] =
        [Topic::Walking, Topic::Greenery, Topic::Cycling, Topic::Transit, Topic::Roadway, Topic::Calming, Topic::Furniture, Topic::Utilities];

    /// Its id, one of `shared::catalogue::GROUP_IDS`.
    pub fn id(self) -> &'static str {
        match self {
            Topic::Walking => "walking",
            Topic::Greenery => "greenery",
            Topic::Cycling => "cycling",
            Topic::Transit => "transit",
            Topic::Roadway => "roadway",
            Topic::Calming => "calming",
            Topic::Furniture => "furniture",
            Topic::Utilities => "utilities",
        }
    }

    /// The message that names it.
    pub fn name_key(self) -> String {
        group_key(self.id())
    }

    /// The catalogue kinds of its pieces, in the menu's order.
    fn piece_ids(self) -> &'static [&'static str] {
        match self {
            Topic::Walking => &["sidewalk"],
            Topic::Greenery => &["planting", "median"],
            Topic::Cycling => &["bike", "bikerack", "bikeshare"],
            Topic::Transit => &["bus", "busshelter", "busstation"],
            Topic::Roadway => &["travel", "parking", "loading", "shoulder", "buffer"],
            Topic::Calming => &[],
            Topic::Furniture => &["bench", "terrace"],
            Topic::Utilities => &["pole", "streetlamp"],
        }
    }
}

/// Where an item goes, which also groups a topic's items into runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// After the selected piece.
    Pieces,
    /// At the first free place along the street.
    Green,
    /// Into the chosen free stretch of a stops row.
    Stops,
    /// Into the chosen free stretch of the raised elements' strip.
    Calming,
}

impl Run {
    pub fn id(self) -> &'static str {
        match self {
            Run::Pieces => "pieces",
            Run::Green => "green",
            Run::Stops => "stops",
            Run::Calming => "calming",
        }
    }

    /// The message that says where its items go.
    pub fn hint_key(self) -> &'static str {
        match self {
            Run::Pieces => "add-hint",
            Run::Green => "add-hint-green",
            Run::Stops | Run::Calming => "add-hint-stretch",
        }
    }

    /// Why its items cannot be added to the street of `v`, or `None` when they can (a piece always can).
    pub fn unavailable(self, v: &View) -> Option<Said> {
        match self {
            Run::Pieces => None,
            Run::Green => v.unavailable.green.clone(),
            Run::Stops => v.unavailable.stops.clone(),
            Run::Calming => v.unavailable.calming.clone(),
        }
    }
}

/// One thing the menu adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// A piece of the catalogue, by its index in `KINDS`.
    Piece(usize),
    Green(GreenKind),
    Stop(StopKind),
    Calming(CalmingKind),
}

impl Item {
    pub fn run(self) -> Run {
        match self {
            Item::Piece(_) => Run::Pieces,
            Item::Green(_) => Run::Green,
            Item::Stop(_) => Run::Stops,
            Item::Calming(_) => Run::Calming,
        }
    }

    /// The message that names it.
    pub fn name_key(self) -> String {
        match self {
            Item::Piece(k) => kind_key(KINDS[k].id),
            Item::Green(k) => k.name_key().to_string(),
            Item::Stop(k) => k.name_key().to_string(),
            Item::Calming(k) => k.name_key().to_string(),
        }
    }

    /// The size it is added at, as the menu shows it: a piece's width, a stop's or calming element's length; a green
    /// element shows none.
    pub fn size_mm(self) -> Option<i32> {
        match self {
            Item::Piece(k) => Some(KINDS[k].default_mm),
            Item::Green(_) => None,
            Item::Stop(_) => Some(crate::standards::model::stop_min_mm()),
            Item::Calming(k) => Some(k.default_mm()),
        }
    }
}

/// A topic's items, in the menu's order: its pieces, then its other things. (A test holds that every kind named is in
/// the catalogue.)
pub fn topic_items(topic: Topic) -> Vec<Item> {
    let pieces = topic.piece_ids().iter().filter_map(|id| kind_index(id)).map(Item::Piece);
    let others: Vec<Item> = match topic {
        Topic::Greenery => GreenKind::ALL.map(Item::Green).to_vec(),
        Topic::Transit => StopKind::ALL.map(Item::Stop).to_vec(),
        Topic::Calming => CalmingKind::ALL.map(Item::Calming).to_vec(),
        _ => Vec::new(),
    };
    pieces.chain(others).collect()
}

/// The topic that holds an item.
pub fn topic_of(item: Item) -> Topic {
    match item {
        Item::Piece(k) => Topic::ALL.into_iter().find(|t| t.piece_ids().contains(&KINDS[k].id)).unwrap_or(Topic::Walking),
        Item::Green(_) => Topic::Greenery,
        Item::Stop(_) => Topic::Transit,
        Item::Calming(_) => Topic::Calming,
    }
}

/// The topic the menu opens on: that of what the details panel shows at `level`, else the first.
pub fn opening_topic(level: Level, v: &View) -> Topic {
    let piece = |k: usize| topic_of(Item::Piece(k));
    match level {
        Level::Piece => v.selected.and_then(|u| v.segments.iter().find(|s| s.uid == u)).map_or(Topic::Walking, |s| piece(s.kind)),
        Level::Stop => Topic::Transit,
        Level::Calming => Topic::Calming,
        Level::Green => Topic::Greenery,
        Level::Curb => v.curb.sides.iter().find_map(|s| s.cells.get(s.selected?)?.use_).and_then(|u| kind_index(u.kind_id())).map_or(Topic::Walking, piece),
        Level::Street => Topic::Walking,
    }
}

/// Where a new piece goes: after the selected one, or at the end.
pub fn insert_index(v: &View) -> usize {
    v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)).map_or(v.segments.len(), |i| i + 1)
}

/// One item as the menu shows it: greyed, with the reason, when it cannot be added to the street as it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub item: Item,
    pub topic: Topic,
    pub unavailable: Option<Said>,
}

fn entry(v: &View, item: Item) -> Entry {
    Entry { item, topic: topic_of(item), unavailable: item.run().unavailable(v) }
}

/// A topic's items on the street of `v`.
pub fn entries(v: &View, topic: Topic) -> Vec<Entry> {
    topic_items(topic).into_iter().map(|item| entry(v, item)).collect()
}

/// Items that go to the same place, under the hint that says where, and why the street cannot take them, if so.
#[derive(Clone, Debug, PartialEq)]
pub struct RunView {
    pub run: Run,
    pub entries: Vec<Entry>,
    pub unavailable: Option<Said>,
}

/// `entries` grouped by where they go, in the order each place is first met, every entry kept in its order: a topic's
/// pieces then its other things, or the places the search's matches go.
pub fn runs(entries: &[Entry]) -> Vec<RunView> {
    let mut out: Vec<RunView> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|r| r.run == e.item.run()) {
            Some(r) => r.entries.push(e.clone()),
            None => out.push(RunView { run: e.item.run(), entries: vec![e.clone()], unavailable: e.unavailable.clone() }),
        }
    }
    out
}

/// Every item whose name in `i18n`'s language holds `query` (both folded), in topic order; empty for a blank query.
/// Names are read with the tracked `tr`, so a language switch searches again.
pub fn search(v: &View, i18n: &I18n, query: &str) -> Vec<Entry> {
    let query = fold(query.trim());
    if query.is_empty() {
        return Vec::new();
    }
    Topic::ALL.into_iter().flat_map(|t| entries(v, t)).filter(|e| fold(&i18n.tr(&e.item.name_key(), &Args::new())).contains(&query)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;

    fn en(key: &str) -> String {
        crate::i18n_for(Locale::En).tr_now(key, &Args::new())
    }

    #[test]
    fn every_kind_stop_green_and_calming_element_is_in_exactly_one_topic() {
        let all: Vec<Item> = Topic::ALL.iter().flat_map(|&t| topic_items(t)).collect();
        assert_eq!(all.len(), 18 + 3 + 4 + 4);
        for (i, a) in all.iter().enumerate() {
            assert!(!all[i + 1..].contains(a), "{a:?} twice");
        }
        for k in 0..KINDS.len() {
            assert!(all.contains(&Item::Piece(k)), "{}", KINDS[k].id);
        }
        for k in StopKind::ALL {
            assert!(all.contains(&Item::Stop(k)));
        }
        for k in GreenKind::ALL {
            assert!(all.contains(&Item::Green(k)));
        }
        for k in CalmingKind::ALL {
            assert!(all.contains(&Item::Calming(k)));
        }
        for t in Topic::ALL {
            assert!(topic_items(t).into_iter().all(|i| topic_of(i) == t), "{t:?}");
        }
    }

    #[test]
    fn the_topics_hold_what_the_design_lists_in_its_order() {
        let names = |t: Topic| topic_items(t).into_iter().map(|i| en(&i.name_key())).collect::<Vec<_>>();
        assert_eq!(names(Topic::Walking), ["Sidewalk"]);
        assert_eq!(names(Topic::Greenery), ["Planting strip", "Planted median", "Tree", "Planting bed", "Rain garden", "Bioswale"]);
        assert_eq!(names(Topic::Cycling), ["Bike lane", "Bike rack", "Bikeshare station"]);
        assert_eq!(names(Topic::Transit), ["Transit lane", "Bus shelter", "Bus station", "Curb stop", "Curb extension", "Boarding island"]);
        assert_eq!(names(Topic::Roadway), ["Driving lane", "Parking", "Loading zone", "Shoulder", "Buffer"]);
        assert_eq!(names(Topic::Calming), ["Raised table", "Raised crossing", "Chicane", "Modal filter"]);
        assert_eq!(names(Topic::Furniture), ["Bench", "Café terrace"]);
        assert_eq!(names(Topic::Utilities), ["Utility pole", "Street lamp"]);
        assert_eq!(Topic::ALL.map(Topic::id), crate::shared::catalogue::GROUP_IDS);
        assert_eq!(Topic::ALL.map(|t| en(&t.name_key()))[5], "Traffic calming");
    }

    #[test]
    fn each_item_says_where_it_goes_and_at_what_size() {
        let bike = kind_index("bike").unwrap();
        assert_eq!(Item::Piece(bike).run(), Run::Pieces);
        assert_eq!(Item::Piece(bike).size_mm(), Some(KINDS[bike].default_mm));
        assert_eq!(Item::Stop(StopKind::Curb).size_mm(), Some(crate::standards::model::stop_min_mm()));
        assert_eq!(Item::Calming(CalmingKind::Chicane).size_mm(), Some(CalmingKind::Chicane.default_mm()));
        assert_eq!(Item::Green(GreenKind::Tree).size_mm(), None);
        assert_eq!([Run::Pieces, Run::Green, Run::Stops, Run::Calming].map(Run::id), ["pieces", "green", "stops", "calming"]);
        assert_eq!(
            [Run::Pieces, Run::Green, Run::Stops, Run::Calming].map(|r| en(r.hint_key())),
            [
                "Goes after the selected piece.",
                "Goes at the first free place along the street.",
                "Goes into the chosen free stretch of its row.",
                "Goes into the chosen free stretch of its row."
            ]
        );
    }

    #[test]
    fn items_are_grouped_into_runs_by_where_they_go_in_the_order_first_met() {
        let reason = Said::new("add-stops-no-lane");
        let piece = |id: &str| Entry { item: Item::Piece(kind_index(id).unwrap()), topic: Topic::Transit, unavailable: None };
        let stop = |k: StopKind| Entry { item: Item::Stop(k), topic: Topic::Transit, unavailable: Some(reason.clone()) };
        let tree = Entry { item: Item::Green(GreenKind::Tree), topic: Topic::Greenery, unavailable: None };
        // A topic: its pieces, then its stops, which carry their reason.
        let topic = [piece("bus"), piece("busshelter"), stop(StopKind::Curb), stop(StopKind::Island)];
        let r = runs(&topic);
        assert_eq!(r.iter().map(|r| r.run).collect::<Vec<_>>(), [Run::Pieces, Run::Stops]);
        assert_eq!(r[0].entries, topic[..2]);
        assert_eq!(r[0].unavailable, None);
        assert_eq!(r[1].entries, topic[2..]);
        assert_eq!(r[1].unavailable, Some(reason.clone()));
        // The search's matches mix them: one run for each place, every match kept in its order.
        let found = [piece("planting"), tree.clone(), piece("bus"), stop(StopKind::Curb)];
        let r = runs(&found);
        assert_eq!(r.iter().map(|r| r.run).collect::<Vec<_>>(), [Run::Pieces, Run::Green, Run::Stops]);
        assert_eq!(r[0].entries, [found[0].clone(), found[2].clone()]);
        assert!(runs(&[]).is_empty());
    }

    #[test]
    fn a_new_piece_goes_after_the_selected_one_or_at_the_end() {
        let mut e = Editor::new(0);
        assert_eq!(insert_index(&e.view()), 6);
        let u = e.view().segments[2].uid;
        e.select(Some(u));
        assert_eq!(insert_index(&e.view()), 3);
    }
}
