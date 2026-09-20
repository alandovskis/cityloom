//! `grid` — the fixed 0.01° grid (`GridSpec`, `entities.md`), cell ids, the
//! touched-cell span of a canonical box (BR3.3, PD-1) and the coverage
//! bitmap (BR4.1). Pure: integer arithmetic only.
//!
//! A cell's column is `floor(lon / cell)` and its row `floor(lat / cell)` in
//! integer units, so cell edges are exact and a box on an edge is
//! unambiguous. Cell ids order row-major, which is also the order the store
//! is written and verified in (PD-7).

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::geo::{CanonicalBox, Extent, UNITS_PER_DEGREE, Units};

/// The default cell size: 0.01° = 1,000 units (NFR1.1.5).
pub const DEFAULT_CELL_UNITS: i64 = 1_000;
/// The provisional span bound: a box may touch at most this many cells
/// (NFR1.1.5; the runtime value is configuration with this default).
pub const SPAN_BOUND_DEFAULT: u64 = 12;

const COL_OFFSET: i64 = 180 * UNITS_PER_DEGREE / DEFAULT_CELL_UNITS * 10; // generous: any divisor of 1 fits
const ROW_OFFSET: i64 = COL_OFFSET;

/// A `GridSpec` text that is not a positive divisor of one degree.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("cell size must be a positive decimal that divides one degree exactly")]
pub struct InvalidGridSpec;

/// How the plane is cut into cells: a regular WGS84 grid anchored at (0, 0).
/// Serialised as `{ "cellSizeDegrees": "0.01", "coordinateSystem": "WGS84" }`;
/// the size is a decimal string so it round-trips exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridSpec {
    cell_units: i64,
}

impl GridSpec {
    /// The 0.01° grid.
    pub fn default_cells() -> GridSpec {
        GridSpec {
            cell_units: DEFAULT_CELL_UNITS,
        }
    }

    /// From a decimal text such as `0.01`; must divide one degree exactly.
    pub fn from_cell_size(text: &str) -> Result<GridSpec, InvalidGridSpec> {
        let b = CanonicalBox::parse(&format!("0,0,{text},1")).map_err(|_| InvalidGridSpec)?;
        let units = b.max_lon.0;
        if units <= 0 || UNITS_PER_DEGREE % units != 0 {
            return Err(InvalidGridSpec);
        }
        // Reject a size that only ceils to an exact divisor (0.009999...).
        if crate::geo::format_units(Units(units))
            .trim_end_matches('0')
            .trim_end_matches('.')
            != text.trim_end_matches('0').trim_end_matches('.')
        {
            return Err(InvalidGridSpec);
        }
        Ok(GridSpec { cell_units: units })
    }

    /// The cell size in units of 1e-5 degree.
    pub fn cell_units(&self) -> i64 {
        self.cell_units
    }

    /// The cell size as the decimal text it is serialised as.
    pub fn cell_size_text(&self) -> String {
        let text = crate::geo::format_units(Units(self.cell_units));
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }

    /// The cell containing a point.
    pub fn cell_of(&self, lon: Units, lat: Units) -> CellId {
        CellId::new(self.col_of(lon), self.row_of(lat))
    }

    fn col_of(&self, lon: Units) -> i32 {
        lon.floor_div(self.cell_units) as i32
    }

    fn row_of(&self, lat: Units) -> i32 {
        lat.floor_div(self.cell_units) as i32
    }

    /// The cells a box touches by area: the maximum edges are exclusive, so a
    /// box ending exactly on a cell edge does not touch the next cell.
    pub fn touched(&self, b: &CanonicalBox) -> TouchedCells {
        TouchedCells {
            col_min: self.col_of(b.min_lon),
            col_max: self.col_of(Units(b.max_lon.0 - 1)),
            row_min: self.row_of(b.min_lat),
            row_max: self.row_of(Units(b.max_lat.0 - 1)),
        }
    }

    /// The cells an extent touches by its points (closed on every edge), so
    /// a way is written into every cell one of its nodes could lie in (BR8.3).
    pub fn touched_by_extent(&self, e: &Extent) -> TouchedCells {
        TouchedCells {
            col_min: self.col_of(e.min_lon),
            col_max: self.col_of(e.max_lon),
            row_min: self.row_of(e.min_lat),
            row_max: self.row_of(e.max_lat),
        }
    }
}

impl Serialize for GridSpec {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GridSpecWire {
            cell_size_degrees: self.cell_size_text(),
            coordinate_system: "WGS84".to_string(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GridSpec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<GridSpec, D::Error> {
        let wire = GridSpecWire::deserialize(deserializer)?;
        if wire.coordinate_system != "WGS84" {
            return Err(serde::de::Error::custom("coordinateSystem must be WGS84"));
        }
        GridSpec::from_cell_size(&wire.cell_size_degrees).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridSpecWire {
    cell_size_degrees: String,
    coordinate_system: String,
}

/// A cell identifier derived from column and row (row-major order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellId(u64);

impl CellId {
    pub fn new(col: i32, row: i32) -> CellId {
        let col_index = (i64::from(col) + COL_OFFSET) as u64;
        let row_index = (i64::from(row) + ROW_OFFSET) as u64;
        CellId((row_index << 32) | col_index)
    }

    pub fn from_raw(raw: u64) -> CellId {
        CellId(raw)
    }

    pub fn raw(self) -> u64 {
        self.0
    }

    pub fn col(self) -> i32 {
        ((self.0 & 0xFFFF_FFFF) as i64 - COL_OFFSET) as i32
    }

    pub fn row(self) -> i32 {
        ((self.0 >> 32) as i64 - ROW_OFFSET) as i32
    }
}

/// The rectangle of cells a box or extent touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchedCells {
    col_min: i32,
    col_max: i32,
    row_min: i32,
    row_max: i32,
}

impl TouchedCells {
    /// How many cells the rectangle holds (the span, BR3.3).
    pub fn count(&self) -> u64 {
        let cols =
            u64::try_from(i64::from(self.col_max) - i64::from(self.col_min) + 1).unwrap_or(0);
        let rows =
            u64::try_from(i64::from(self.row_max) - i64::from(self.row_min) + 1).unwrap_or(0);
        cols * rows
    }

    /// The cells in cell-id (row-major) order.
    pub fn iter(&self) -> impl Iterator<Item = CellId> + '_ {
        (self.row_min..=self.row_max).flat_map(move |row| {
            (self.col_min..=self.col_max).map(move |col| CellId::new(col, row))
        })
    }

    /// The rectangle as the serialisable shape the manifest carries.
    pub fn rect(&self) -> CellRect {
        CellRect {
            col_min: self.col_min,
            col_max: self.col_max,
            row_min: self.row_min,
            row_max: self.row_max,
        }
    }
}

/// A rectangle of cells, the manifest's representation of coverage
/// (`CoverageIndex.coveredCellIds` is the union of these).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellRect {
    pub col_min: i32,
    pub col_max: i32,
    pub row_min: i32,
    pub row_max: i32,
}

/// The set of covered cells as a bitmap over the regions' bounding rectangle
/// (SC-4); derived at build time from the regions' bounds, never edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageIndex {
    col_min: i32,
    row_min: i32,
    cols: u64,
    rows: u64,
    bits: Vec<u64>,
}

impl CoverageIndex {
    pub fn from_rectangles(rects: &[CellRect]) -> CoverageIndex {
        let Some(first) = rects.first() else {
            return CoverageIndex {
                col_min: 0,
                row_min: 0,
                cols: 0,
                rows: 0,
                bits: Vec::new(),
            };
        };
        let (mut col_min, mut col_max, mut row_min, mut row_max) =
            (first.col_min, first.col_max, first.row_min, first.row_max);
        for r in rects {
            col_min = col_min.min(r.col_min);
            col_max = col_max.max(r.col_max);
            row_min = row_min.min(r.row_min);
            row_max = row_max.max(r.row_max);
        }
        let cols = u64::try_from(i64::from(col_max) - i64::from(col_min) + 1).unwrap_or(0);
        let rows = u64::try_from(i64::from(row_max) - i64::from(row_min) + 1).unwrap_or(0);
        let mut index = CoverageIndex {
            col_min,
            row_min,
            cols,
            rows,
            bits: vec![0; (cols * rows).div_ceil(64) as usize],
        };
        for r in rects {
            for row in r.row_min..=r.row_max {
                for col in r.col_min..=r.col_max {
                    if let Some(bit) = index.bit_of(col, row)
                        && let Some(word) = index.bits.get_mut((bit / 64) as usize)
                    {
                        *word |= 1u64 << (bit % 64);
                    }
                }
            }
        }
        index
    }

    fn bit_of(&self, col: i32, row: i32) -> Option<u64> {
        let c = i64::from(col) - i64::from(self.col_min);
        let r = i64::from(row) - i64::from(self.row_min);
        if c < 0 || r < 0 || c as u64 >= self.cols || r as u64 >= self.rows {
            return None;
        }
        Some(r as u64 * self.cols + c as u64)
    }

    /// Whether one cell is covered.
    pub fn covers(&self, cell: CellId) -> bool {
        self.bit_of(cell.col(), cell.row())
            .and_then(|bit| {
                self.bits
                    .get((bit / 64) as usize)
                    .map(|word| word & (1u64 << (bit % 64)) != 0)
            })
            .unwrap_or(false)
    }

    /// Whether every touched cell is covered (BR4.1).
    pub fn covers_all(&self, touched: &TouchedCells) -> bool {
        touched.iter().all(|cell| self.covers(cell))
    }

    /// The bitmap's size in bytes (NFR2.1.5).
    pub fn bytes(&self) -> usize {
        self.bits.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{CanonicalBox, Extent, Units};

    fn grid() -> GridSpec {
        GridSpec::default_cells()
    }

    fn cells(t: &TouchedCells) -> Vec<(i32, i32)> {
        t.iter().map(|c| (c.col(), c.row())).collect()
    }

    // NFR1.1.5 — 0.01° cells are exactly 1,000 units of 1e-5 degree.
    #[test]
    fn a_cell_is_exactly_one_thousand_units() {
        assert_eq!(grid().cell_units(), 1_000);
        assert_eq!(
            GridSpec::from_cell_size("0.01").unwrap().cell_units(),
            1_000
        );
        assert!(
            GridSpec::from_cell_size("0.03").is_err(),
            "0.03 does not divide 1"
        );
        assert!(GridSpec::from_cell_size("0").is_err());
        assert!(GridSpec::from_cell_size("abc").is_err());
    }

    #[test]
    fn column_and_row_come_from_integer_division() {
        let g = grid();
        assert_eq!(
            g.cell_of(Units(-7_963_000), Units(4_365_000)),
            CellId::new(-7963, 4365)
        );
        assert_eq!(
            g.cell_of(Units(-7_962_999), Units(4_365_999)),
            CellId::new(-7963, 4365)
        );
        assert_eq!(
            g.cell_of(Units(-7_962_000), Units(4_366_000)),
            CellId::new(-7962, 4366)
        );
        // Exact limits stay inside the id space.
        assert_eq!(
            g.cell_of(Units(-18_000_000), Units(-9_000_000)),
            CellId::new(-18000, -9000)
        );
        assert_eq!(
            g.cell_of(Units(17_999_999), Units(8_999_999)),
            CellId::new(17999, 8999)
        );
    }

    #[test]
    fn cell_ids_are_stable_and_ordered_row_major() {
        let a = CellId::new(-7963, 4365);
        assert_eq!((a.col(), a.row()), (-7963, 4365));
        assert_eq!(CellId::from_raw(a.raw()), a);
        // Row-major: a lower row sorts first; within a row, a lower column.
        assert!(CellId::new(5, 1) < CellId::new(-100, 2));
        assert!(CellId::new(-100, 2) < CellId::new(5, 2));
    }

    // NFR1.1.5 — a box straddling a cell corner touches four cells.
    #[test]
    fn a_box_straddling_a_corner_touches_four_cells() {
        let b = CanonicalBox::parse("-79.6305,43.6495,-79.6295,43.6505").unwrap();
        let t = grid().touched(&b);
        assert_eq!(t.count(), 4);
        assert_eq!(
            cells(&t),
            vec![(-7964, 4364), (-7963, 4364), (-7964, 4365), (-7963, 4365)]
        );
    }

    // NFR1.1.5 — a box exactly on a cell edge touches only what it overlaps by area.
    #[test]
    fn a_box_on_an_edge_touches_only_what_it_overlaps_by_area() {
        let b = CanonicalBox::parse("-79.63,43.65,-79.62,43.66").unwrap();
        let t = grid().touched(&b);
        assert_eq!(t.count(), 1);
        assert_eq!(cells(&t), vec![(-7963, 4365)]);
        // One unit past the edge touches the next cell too.
        let b = CanonicalBox::parse("-79.63,43.65,-79.61999,43.66").unwrap();
        assert_eq!(grid().touched(&b).count(), 2);
    }

    // BR3.3 / NFR1.1.5 — a 13-cell rectangle exceeds the span bound of 12.
    #[test]
    fn a_thirteen_cell_rectangle_exceeds_the_span_bound() {
        let twelve = CanonicalBox::parse("0,0,0.04,0.03").unwrap(); // 4 x 3
        assert_eq!(grid().touched(&twelve).count(), 12);
        assert!(grid().touched(&twelve).count() <= SPAN_BOUND_DEFAULT);
        let thirteen = CanonicalBox::parse("0,0,0.13,0.01").unwrap(); // 13 x 1
        assert_eq!(grid().touched(&thirteen).count(), 13);
        assert!(grid().touched(&thirteen).count() > SPAN_BOUND_DEFAULT);
    }

    #[test]
    fn an_extent_touches_cells_by_its_points() {
        // A way whose extent ends exactly on a cell edge is in that cell too.
        let e =
            Extent::of_points([(150_000_000, 250_000_000), (200_000_000, 250_000_000)]).unwrap();
        let t = grid().touched_by_extent(&e);
        assert_eq!(
            cells(&t),
            vec![(15, 25), (16, 25), (17, 25), (18, 25), (19, 25), (20, 25)]
        );
    }

    // BR4.1 — a box is covered only when every touched cell is covered.
    #[test]
    fn coverage_answers_covered_and_uncovered() {
        let region_a = grid().touched(&CanonicalBox::parse("-63.20,46.20,-63.10,46.30").unwrap());
        let region_b = grid().touched(&CanonicalBox::parse("-63.00,46.20,-62.90,46.30").unwrap());
        let index = CoverageIndex::from_rectangles(&[region_a.rect(), region_b.rect()]);
        assert!(index.covers(CellId::new(-6320, 4620)));
        assert!(index.covers(CellId::new(-6311, 4629)));
        assert!(index.covers(CellId::new(-6295, 4625)));
        assert!(
            !index.covers(CellId::new(-6305, 4625)),
            "the gap between the regions"
        );
        assert!(!index.covers(CellId::new(-6321, 4620)), "west of region A");
        assert!(
            !index.covers(CellId::new(-6310, 4620)),
            "east edge of region A is exclusive"
        );
        assert!(
            !index.covers(CellId::new(0, 0)),
            "outside the rectangle entirely"
        );

        let inside = grid().touched(&CanonicalBox::parse("-63.195,46.205,-63.185,46.215").unwrap());
        assert!(index.covers_all(&inside));
        let straddling =
            grid().touched(&CanonicalBox::parse("-63.105,46.205,-63.095,46.215").unwrap());
        assert_eq!(straddling.count(), 4);
        assert!(
            !index.covers_all(&straddling),
            "one uncovered cell makes the box uncovered"
        );
    }

    #[test]
    fn coverage_bitmap_is_small() {
        let region = grid().touched(&CanonicalBox::parse("-139.0,48.3,-114.0,60.0").unwrap());
        let index = CoverageIndex::from_rectangles(&[region.rect()]);
        assert!(
            index.bytes() < 1_000_000,
            "NFR2.1.5: under 1 MB for a province-sized rectangle"
        );
    }
}
