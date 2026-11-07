//! The PDF writer: Letter pages, Noto Sans, greedy word wrapping, and a structure tree (Document > H / P / Table).

use std::num::NonZeroU16;
use std::sync::Arc;

use krilla::color::rgb;
use krilla::configure::{Accessibility, ConfigurationBuilder};
use krilla::destination::XyzDestination;
use krilla::geom::{PathBuilder, Point, Rect};
use krilla::metadata::Metadata;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::tagging::{ContentTag, Node, SpanTag, TableHeaderScope, Tag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};
use krilla::{Document, SerializeSettings};
use wasm_bindgen::prelude::*;

use cityloom_report::{Block, Captioned, Item, ReportData, Rgb};

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;
const MARGIN: f32 = 54.0;
const BODY: f32 = 11.0;
const LEADING: f32 = 1.35;
const GAP: f32 = 8.0;
const CELL_PAD: f32 = 4.0;
const HALF: f32 = (PAGE_W - 2.0 * MARGIN - FIGURE_GAP) / 2.0;
const FIGURE_GAP: f32 = 12.0;
const CAPTION: f32 = 10.0;
const NOTE: f32 = 8.0;
/// The thinnest stroke a figure is drawn with, in points.
const HAIRLINE: f32 = 0.25;

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

    /// One or two figures side by side, each in half the text width with its caption above and its note below.
    /// The block is scaled down if it would not fit what is left of the page.
    fn figures(&mut self, items: &[Captioned]) -> Vec<Node> {
        let top = self.y;
        let mut end = top;
        let mut nodes = vec![];
        for (slot, item) in items.iter().take(2).enumerate() {
            let x = MARGIN + slot as f32 * (HALF + FIGURE_GAP);
            self.y = top;
            let caption = wrap(self.bold, &item.caption, CAPTION, HALF);
            let note = wrap(self.regular, &item.note, NOTE, HALF);
            let lines_h = |n: usize, size: f32| n as f32 * size * LEADING;
            let room = self.bottom() - top - lines_h(caption.len(), CAPTION) - lines_h(note.len(), NOTE) - 2.0 * GAP;
            let (fw, fh) = (item.figure.width.max(f32::EPSILON), item.figure.height.max(f32::EPSILON));
            let scale = (HALF / fw).min(room.max(1.0) / fh);
            let nodes_caption = self.lines(&caption, x, CAPTION, true);
            if !nodes_caption.is_empty() {
                nodes.push(Node::Group(TagGroup::with_children(Tag::P, nodes_caption)));
            }
            self.y += GAP / 2.0;
            let (box_w, box_h) = (fw * scale, fh * scale);
            if !item.figure.items.is_empty() {
                let id = self.surface.start_tagged(ContentTag::Other);
                self.draw_figure(item, x, self.y, scale, box_w, box_h);
                self.surface.end_tagged();
                nodes.push(Node::Group(TagGroup::with_children(Tag::Figure(Some(item.alt.clone())), vec![Node::from(id)])));
            }
            self.y += box_h + GAP / 2.0;
            let nodes_note = self.lines(&note, x, NOTE, false);
            if !nodes_note.is_empty() {
                nodes.push(Node::Group(TagGroup::with_children(Tag::P, nodes_note)));
            }
            end = end.max(self.y);
        }
        self.y = end + GAP;
        nodes
    }

    /// Draws a figure's items with its box's corner at (`x`, `y`), clipped to the box; lengths scale by `scale`.
    fn draw_figure(&mut self, item: &Captioned, x: f32, y: f32, scale: f32, box_w: f32, box_h: f32) {
        let colour = |c: Rgb| rgb::Color::new(c.0, c.1, c.2);
        let at = |p: &(f32, f32)| (x + p.0 * scale, y + p.1 * scale);
        let path = |pts: &[(f32, f32)], close: bool| {
            let mut b = PathBuilder::new();
            for (i, p) in pts.iter().enumerate() {
                let (px, py) = at(p);
                if i == 0 { b.move_to(px, py) } else { b.line_to(px, py) }
            }
            if close {
                b.close();
            }
            b.finish()
        };
        if let Some(clip) = Rect::from_xywh(x, y, box_w, box_h).and_then(|r| {
            let mut b = PathBuilder::new();
            b.push_rect(r);
            b.finish()
        }) {
            self.surface.push_clip_path(&clip, &krilla::paint::FillRule::default());
        }
        let stroke = |c: Rgb, w: f32| Stroke { paint: colour(c).into(), width: (w * scale).max(HAIRLINE), ..Stroke::default() };
        for it in &item.figure.items {
            match it {
                Item::Poly { pts, fill, stroke: outline } => {
                    if let Some(p) = path(pts, true) {
                        self.surface.set_fill(Some(Fill { paint: colour(*fill).into(), ..Fill::default() }));
                        self.surface.set_stroke(outline.map(|(c, w)| stroke(c, w)));
                        self.surface.draw_path(&p);
                    }
                }
                Item::Line { pts, color, width } => {
                    if let Some(p) = path(pts, false) {
                        self.surface.set_fill(None);
                        self.surface.set_stroke(Some(stroke(*color, *width)));
                        self.surface.draw_path(&p);
                    }
                }
                Item::Text { at: pos, size, color, text } => {
                    let text = self.regular.drawable(text);
                    let (px, py) = at(pos);
                    self.surface.set_stroke(None);
                    self.surface.set_fill(Some(Fill { paint: colour(*color).into(), ..Fill::default() }));
                    self.surface.draw_text(Point::from_xy(px, py), self.regular.font.clone(), size * scale, &text, false, TextDirection::Auto);
                }
            }
        }
        self.surface.set_stroke(None);
        self.surface.set_fill(None);
        self.surface.pop();
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
                    Block::Figures { items } => Block::Figures {
                        items: items
                            .iter()
                            .map(|c| Captioned { caption: clean(&c.caption), alt: c.alt.clone(), note: clean(&c.note), figure: c.figure.clone() })
                            .collect(),
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
                Block::Figures { items } => {
                    for node in pen.figures(items) {
                        tree.push(node);
                    }
                    None
                }
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
