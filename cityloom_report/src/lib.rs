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
    Heading { level: u8, text: String },
    Paragraph { text: String },
    Table { caption: String, head: Vec<String>, rows: Vec<Vec<String>> },
}
