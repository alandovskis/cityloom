//! The PDF writer: Letter pages, Noto Sans, greedy word wrapping, and a structure tree (Document > H / P / Table).

use std::num::NonZeroU16;
use std::sync::Arc;

use krilla::Document;
use krilla::geom::Point;
use krilla::metadata::Metadata;
use krilla::page::PageSettings;
use krilla::surface::Surface;
use krilla::tagging::{ContentTag, Node, SpanTag, TableHeaderScope, Tag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};
use wasm_bindgen::prelude::*;

use crate::{Block, ReportData};

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;
const MARGIN: f32 = 54.0;
const BODY: f32 = 11.0;
const LEADING: f32 = 1.35;
const GAP: f32 = 8.0;
const CELL_PAD: f32 = 4.0;

/// A font with the bytes needed to wrap text.
struct Face {
    font: Font,
    bytes: Vec<u8>,
}

impl Face {
    fn new(bytes: &[u8]) -> Result<Face, String> {
        let font = Font::new(Arc::new(bytes.to_vec()).into(), 0).ok_or("a font cannot be read")?;
        ttf_parser::Face::parse(bytes, 0).map_err(|e| e.to_string())?;
        Ok(Face { font, bytes: bytes.to_vec() })
    }

    /// The width of `text` at `size` points, from the advance of each glyph (no kerning, so never narrower than drawn).
    fn width(&self, text: &str, size: f32) -> f32 {
        let face = ttf_parser::Face::parse(&self.bytes, 0).expect("checked in new");
        let per_em = f32::from(face.units_per_em());
        text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(0.0, f32::from) * size / per_em).sum()
    }
}

/// The text broken into lines no wider than `max` points: at spaces, and inside a word that is wider than a line.
fn wrap(face: &Face, text: &str, size: f32, max: f32) -> Vec<String> {
    let mut lines = vec![];
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let joined = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if face.width(&joined, size) <= max {
                line = joined;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            // A word on a line of its own: cut where it no longer fits.
            for c in word.chars() {
                let mut next = line.clone();
                next.push(c);
                if !line.is_empty() && face.width(&next, size) > max {
                    lines.push(std::mem::take(&mut line));
                    line.push(c);
                } else {
                    line = next;
                }
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

struct Pen<'a, 'b> {
    surface: &'a mut Surface<'b>,
    regular: &'a Face,
    bold: &'a Face,
    y: f32,
}

impl Pen<'_, '_> {
    fn bottom(&self) -> f32 {
        PAGE_H - MARGIN
    }

    /// Draws the lines as tagged text from `x`, one under the other, and returns the tagged pieces; a line that would
    /// cross the bottom margin is dropped (a page that overflows is cut).
    fn lines(&mut self, lines: &[String], x: f32, size: f32, bold: bool) -> Vec<Node> {
        let face = if bold { self.bold } else { self.regular };
        let mut nodes = vec![];
        for line in lines {
            let baseline = self.y + size;
            if baseline > self.bottom() {
                break;
            }
            let id = self.surface.start_tagged(ContentTag::Span(SpanTag::empty()));
            self.surface.draw_text(Point::from_xy(x, baseline), face.font.clone(), size, line, false, TextDirection::Auto);
            self.surface.end_tagged();
            nodes.push(Node::from(id));
            self.y += size * LEADING;
        }
        nodes
    }

    fn text(&mut self, tag: impl Into<krilla::tagging::TagKind>, text: &str, size: f32, bold: bool) -> Option<Node> {
        let face = if bold { self.bold } else { self.regular };
        let lines = wrap(face, text, size, PAGE_W - 2.0 * MARGIN);
        let nodes = self.lines(&lines, MARGIN, size, bold);
        self.y += GAP;
        (!nodes.is_empty()).then(|| Node::Group(TagGroup::with_children(tag, nodes)))
    }

    fn table(&mut self, caption: &str, head: &[String], rows: &[Vec<String>]) -> Option<Node> {
        let mut children = vec![];
        if !caption.is_empty() {
            let lines = wrap(self.bold, caption, BODY, PAGE_W - 2.0 * MARGIN);
            let nodes = self.lines(&lines, MARGIN, BODY, true);
            if !nodes.is_empty() {
                children.push(Node::Group(TagGroup::with_children(Tag::Caption, nodes)));
            }
        }
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0)).max(1);
        let cell_w = (PAGE_W - 2.0 * MARGIN) / columns as f32;
        let all = std::iter::once((head, true)).chain(rows.iter().map(|r| (r.as_slice(), false)));
        for (cells, is_head) in all {
            if cells.is_empty() {
                continue;
            }
            let top = self.y;
            let mut height: f32 = 0.0;
            let mut tr = vec![];
            for (i, cell) in cells.iter().enumerate() {
                self.y = top;
                let face = if is_head { self.bold } else { self.regular };
                let lines = wrap(face, cell, BODY, cell_w - CELL_PAD);
                let nodes = self.lines(&lines, MARGIN + i as f32 * cell_w, BODY, is_head);
                height = height.max(self.y - top);
                if !nodes.is_empty() {
                    let group =
                        if is_head { TagGroup::with_children(Tag::TH(TableHeaderScope::Column), nodes) } else { TagGroup::with_children(Tag::TD, nodes) };
                    tr.push(Node::Group(group));
                }
            }
            self.y = top + height;
            if !tr.is_empty() {
                children.push(Node::Group(TagGroup::with_children(Tag::TR, tr)));
            }
        }
        self.y += GAP;
        (!children.is_empty()).then(|| Node::Group(TagGroup::with_children(Tag::Table, children)))
    }
}

/// Writes the report as a tagged PDF with the given fonts (static TrueType, regular and bold).
pub fn write_pdf(data: &ReportData, regular: &[u8], bold: &[u8]) -> Result<Vec<u8>, String> {
    let regular = Face::new(regular)?;
    let bold = Face::new(bold)?;
    let mut document = Document::new();
    document.set_metadata(Metadata::new().title(data.title.clone()).language(data.lang.clone()));
    let mut tree = TagTree::new().with_lang(Some(data.lang.clone()));
    let size = PageSettings::from_wh(PAGE_W, PAGE_H).ok_or("the page size is invalid")?;
    for blocks in &data.pages {
        let mut page = document.start_page_with(size.clone());
        let mut surface = page.surface();
        let mut pen = Pen { surface: &mut surface, regular: &regular, bold: &bold, y: MARGIN };
        for block in blocks {
            let node = match block {
                Block::Heading { level, text } => {
                    let level = NonZeroU16::new(u16::from((*level).clamp(1, 6))).expect("at least 1");
                    let size = match level.get() {
                        1 => 18.0,
                        2 => 14.0,
                        _ => 12.0,
                    };
                    pen.text(Tag::Hn(level, None), text, size, true)
                }
                Block::Paragraph { text } => pen.text(Tag::P, text, BODY, false),
                Block::Table { caption, head, rows } => pen.table(caption, head, rows),
            };
            if let Some(node) = node {
                tree.push(node);
            }
        }
        surface.finish();
        page.finish();
    }
    document.set_tag_tree(tree);
    document.finish().map_err(|e| format!("{e:?}"))
}

/// The wasm entry: the report as JSON in, the PDF's bytes out.
#[wasm_bindgen]
pub fn report_pdf(json: &str, regular: &[u8], bold: &[u8]) -> Result<Vec<u8>, JsError> {
    let data: ReportData = serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    write_pdf(&data, regular, bold).map_err(|e| JsError::new(&e))
}
