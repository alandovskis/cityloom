//! Which street carries on from the end of another: what the street page's previous and next arrows follow.

use super::{City, Layout};

/// Scores closer than this are a tie (floating-point noise between mirrored angles).
const TIE: f64 = 1e-9;

impl Layout {
    /// The unit direction in which street `edge` leaves `node`, along its centreline; none when the street does
    /// not touch the node or has no length.
    fn heading_at(&self, edge: usize, node: usize) -> Option<(f64, f64)> {
        let e = self.edges.get(edge)?;
        let mut shape = self.shape_of(edge);
        if e.a != node {
            if e.b != node {
                return None;
            }
            shape.reverse();
        }
        let (x0, y0) = *shape.first()?;
        let (x, y) = shape.iter().find(|&&p| p != (x0, y0))?;
        let (dx, dy) = ((x - x0) as f64, (y - y0) as f64);
        let len = dx.hypot(dy);
        Some((dx / len, dy / len))
    }

    /// The street that carries on from `edge` at `node`: among the others meeting there, one of the same name when
    /// there is one, then the one that bends least from `edge`'s line; the first on a tie. None at a dead end or
    /// the edge of the map.
    pub(super) fn continuing(&self, edge: usize, node: usize) -> Option<usize> {
        let others: Vec<usize> = self.edges_at(node).into_iter().filter(|&c| c != edge).collect();
        let name = self.edges[edge].name.as_ref();
        let same: Vec<usize> = others.iter().copied().filter(|&c| name.is_some() && self.edges[c].name.as_ref() == name).collect();
        let pool = if same.is_empty() { others } else { same };
        let here = self.heading_at(edge, node);
        // Going straight on means leaving opposite to the way in: the smallest dot product of the two directions.
        let score = |c: usize| match (here, self.heading_at(c, node)) {
            (Some(h), Some(l)) => h.0 * l.0 + h.1 * l.1,
            _ => 0.0,
        };
        let mut best: Option<(usize, f64)> = None;
        for c in pool {
            let s = score(c);
            if best.is_none_or(|(_, b)| s < b - TIE) {
                best = Some((c, s));
            }
        }
        best.map(|(c, _)| c)
    }
}

impl City {
    /// The streets that carry on from a street's two ends, by uid, in the order of `street_ends` (none at a dead
    /// end, the edge of the map, or for a street not in the city).
    pub fn street_neighbours(&self, edge: u32) -> [Option<u32>; 2] {
        let Some(e) = self.edge_index(edge) else { return [None; 2] };
        let at = |n: usize| self.layout.continuing(e, n).map(|c| c as u32 + 1);
        [at(self.layout.edges[e].a), at(self.layout.edges[e].b)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(names: &[(usize, &str)]) -> Layout {
        let mut layout = Layout::sample();
        for &(e, name) in names {
            layout.edges[e].name = Some(name.to_string());
        }
        layout
    }

    // Sample city, edge indexes (uid - 1): edge 0 runs node 0 (20,340) to node 1 (230,340); at node 1
    // meet edges 1 (east), 4 (north) and 5 (south); node 15 (230,500) has edges 5, 16, 19, 20, 21.

    #[test]
    fn a_street_continues_straight_where_nothing_is_named() {
        let layout = Layout::sample();
        assert_eq!(layout.continuing(0, 1), Some(1), "the avenue goes on east");
        assert_eq!(layout.continuing(1, 1), Some(0), "and back west");
        assert_eq!(layout.continuing(19, 15), Some(16), "the western lane meets the five ways and goes on east");
    }

    #[test]
    fn the_end_of_the_map_has_no_continuation() {
        let layout = Layout::sample();
        assert_eq!(layout.continuing(0, 0), None);
    }

    #[test]
    fn a_street_of_the_same_name_wins_over_a_straighter_one() {
        let layout = named(&[(0, "Main"), (5, "Main"), (1, "Other")]);
        assert_eq!(layout.continuing(0, 1), Some(5), "Main turns south rather than leave as Other");
        let layout = named(&[(0, "Main"), (1, "Other"), (5, "Else")]);
        assert_eq!(layout.continuing(0, 1), Some(1), "no street of the name: the straightest");
    }

    #[test]
    fn a_tie_takes_the_first_street() {
        // Edges 20 and 21 leave node 15 at mirrored angles to a street arriving from the north.
        let layout = Layout::sample();
        assert_eq!(layout.continuing(5, 15), Some(20));
    }

    #[test]
    fn the_city_names_the_streets_at_both_ends_by_uid() {
        let city = City::new();
        assert_eq!(city.street_neighbours(1), [None, Some(2)], "street 1 starts at the edge of the map");
        assert_eq!(city.street_neighbours(2), [Some(1), Some(3)]);
        assert_eq!(city.street_neighbours(999), [None, None]);
    }
}
