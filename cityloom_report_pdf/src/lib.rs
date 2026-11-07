//! The PDF writer: Letter pages, Noto Sans, greedy word wrapping, and a structure tree (Document > H / P / Table).

use std::num::NonZeroU16;
use std::sync::Arc;

use krilla::configure::{Accessibility, ConfigurationBuilder};
use krilla::destination::XyzDestination;
use krilla::geom::Point;
use krilla::metadata::Metadata;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::surface::Surface;
use krilla::tagging::{ContentTag, Node, SpanTag, TableHeaderScope, Tag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};
use krilla::{Document, SerializeSettings};
use wasm_bindgen::prelude::*;

use cityloom_report::{Block, ReportData};

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

    /// `text` with each character the font lacks (a street name in another script) as "?". PDF/UA-1 refuses a file
    /// that draws .notdef, and one such name must not cost the whole report.
    fn drawable(&self, text: &str) -> String {
        let face = ttf_parser::Face::parse(&self.bytes, 0).expect("checked in new");
        text.chars().map(|c| if c == '\n' || face.glyph_index(c).is_some() { c } else { '?' }).collect()
    }

    /// The width of `text` at `size` points, from the advance of each glyph (no kerning, so never narrower than drawn).
    fn width(&self, text: &str, size: f32) -> f32 {
        let face = ttf_parser::Face::parse(&self.bytes, 0).expect("checked in new");
        let per_em = f32::from(face.units_per_em());
        // Text is made drawable first, so every character has a glyph; glyph 0 is only a fallback.
        text.chars().map(|c| f32::from(face.glyph_hor_advance(face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0))).unwrap_or(0)) * size / per_em).sum()
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
                let mut lines = wrap(face, cell, BODY, cell_w - CELL_PAD);
                if lines.is_empty() {
                    // An empty cell still is a cell: it holds one space, since an element with no content is dropped.
                    lines.push(" ".to_string());
                }
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
    // PDF/UA-1 asks for a tagged document and also sets /ViewerPreferences /DisplayDocTitle, so a viewer shows the title.
    let configuration = ConfigurationBuilder::new().with_accessibility_validator(Accessibility::UA1).finish().map_err(|e| format!("{e:?}"))?;
    let mut document = Document::new_with(SerializeSettings { configuration, ..SerializeSettings::default() });
    document.set_metadata(Metadata::new().title(data.title.clone()).language(data.lang.clone()));
    let mut tree = TagTree::new().with_lang(Some(data.lang.clone()));
    let size = PageSettings::from_wh(PAGE_W, PAGE_H).ok_or("the page size is invalid")?;
    let clean = |text: &str| regular.drawable(text);
    let pages: Vec<Vec<Block>> = data
        .pages
        .iter()
        .map(|page| {
            page.iter()
                .map(|block| match block {
                    Block::Heading { level, text } => Block::Heading { level: *level, text: clean(text) },
                    Block::Paragraph { text } => Block::Paragraph { text: clean(text) },
                    Block::Table { caption, head, rows } => Block::Table {
                        caption: clean(caption),
                        head: head.iter().map(|c| clean(c)).collect(),
                        rows: rows.iter().map(|r| r.iter().map(|c| clean(c)).collect()).collect(),
                    },
                })
                .collect()
        })
        .collect();
    let mut outline = Outline::new();
    for (index, blocks) in pages.iter().enumerate() {
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
                    let top = pen.y;
                    let node = pen.text(Tag::Hn(level, Some(text.clone())), text, size, true);
                    if node.is_some() {
                        // PDF/UA-1 wants every heading titled and a bookmark for the document.
                        let destination = XyzDestination::new(index, Point::from_xy(MARGIN, top));
                        outline.push_child(OutlineNode::new(text.clone(), destination));
                    }
                    node
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
    document.set_outline(outline);
    document.set_tag_tree(tree);
    document.finish().map_err(|e| format!("{e:?}"))
}

/// The wasm entry: the report as JSON in, the PDF's bytes out.
#[wasm_bindgen]
pub fn report_pdf(json: &str, regular: &[u8], bold: &[u8]) -> Result<Vec<u8>, JsError> {
    let data: ReportData = serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    write_pdf(&data, regular, bold).map_err(|e| JsError::new(&e))
}
