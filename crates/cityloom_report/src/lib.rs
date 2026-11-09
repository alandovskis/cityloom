//! The design-review report as plain data.
//!
//! The editor builds a [`ReportData`] (the words already said in the person's language) and hands it, as JSON, to the
//! PDF writer (`cityloom_report_pdf`), which is its own wasm module because krilla is large. This crate needs serde only.

use serde::{Deserialize, Serialize};

/// A report: its language (a BCP 47 tag), its title and its pages, each a list of blocks top to bottom.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportData {
    pub lang: String,
    pub title: String,
    pub pages: Vec<Vec<Block>>,
}

/// One block of a page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    Table {
        caption: String,
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// One or two figures side by side, each with its caption above and its note below.
    Figures {
        items: Vec<Captioned>,
    },
}

/// A [`Figure`] with its words: the caption above it, the alternative text a screen reader says for it, and a note
/// under it (may be empty).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Captioned {
    pub caption: String,
    pub alt: String,
    pub note: String,
    pub figure: Figure,
}

/// A colour, 8 bits a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// One flat thing of a [`Figure`], in the figure's own units (y down).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    /// A closed polygon, filled, with an optional outline (colour and width).
    Poly { pts: Vec<(f32, f32)>, fill: Rgb, stroke: Option<(Rgb, f32)> },
    /// An open line.
    Line { pts: Vec<(f32, f32)>, color: Rgb, width: f32 },
    /// A word with its left baseline at `at`.
    Text { at: (f32, f32), size: f32, color: Rgb, text: String },
}

/// A drawing as flat items, painted in order, in a `width` by `height` box. `omitted` counts the shapes the flattening
/// could not draw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    pub width: f32,
    pub height: f32,
    pub items: Vec<Item>,
    pub omitted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_figure_survives_json() {
        let f = Figure {
            width: 10.0,
            height: 5.0,
            items: vec![
                Item::Poly { pts: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)], fill: Rgb(1, 2, 3), stroke: Some((Rgb(4, 5, 6), 1.0)) },
                Item::Line { pts: vec![(0.0, 0.0), (2.0, 2.0)], color: Rgb(0, 0, 0), width: 0.5 },
                Item::Text { at: (1.0, 2.0), size: 8.0, color: Rgb(9, 9, 9), text: "A".into() },
            ],
            omitted: 2,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(serde_json::from_str::<Figure>(&json).unwrap(), f);
    }
}
