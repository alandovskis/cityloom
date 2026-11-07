//! Reads the written files back with lopdf.

use cityloom_report::{Block, ReportData};
use cityloom_report_pdf::write_pdf;
use lopdf::{Document, Object};

const REGULAR: &[u8] = include_bytes!("../../web/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../web/fonts/NotoSans-Bold.ttf");

fn heading(text: &str) -> Block {
    Block::Heading { level: 1, text: text.to_string() }
}

fn para(text: &str) -> Block {
    Block::Paragraph { text: text.to_string() }
}

fn data(pages: Vec<Vec<Block>>) -> ReportData {
    ReportData { lang: "fr-CA".into(), title: "Revue de conception".into(), pages }
}

fn load(d: &ReportData) -> Document {
    let bytes = write_pdf(d, REGULAR, BOLD).expect("the pdf is written");
    assert!(bytes.starts_with(b"%PDF"));
    Document::load_mem(&bytes).expect("lopdf reads it")
}

fn catalog(doc: &Document) -> &lopdf::Dictionary {
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    doc.get_dictionary(root).unwrap()
}

fn page_text(doc: &Document, n: u32) -> String {
    doc.extract_text(&[n]).expect("text is extracted")
}

/// Each text drawing's start (x, y) in the page's content stream, in drawing order.
fn text_starts(doc: &Document, n: u32) -> Vec<(f32, f32)> {
    let id = *doc.get_pages().get(&n).unwrap();
    let content = doc.get_and_decode_page_content(id).unwrap();
    let mut starts = vec![];
    let mut matrix = (0.0, 0.0);
    let mut cm = (0.0, 0.0);
    for op in &content.operations {
        let num = |i: usize| match &op.operands[i] {
            Object::Integer(v) => *v as f32,
            Object::Real(v) => *v,
            _ => 0.0,
        };
        match op.operator.as_str() {
            "cm" if op.operands.len() == 6 => cm = (num(4), num(5)),
            "Tm" if op.operands.len() == 6 => matrix = (num(4), num(5)),
            "Td" if op.operands.len() == 2 => matrix = (num(0), num(1)),
            "Tj" | "TJ" => starts.push((matrix.0 + cm.0, matrix.1)),
            _ => {}
        }
    }
    starts
}

#[test]
fn it_writes_one_pdf_page_per_page_of_data() {
    let doc = load(&data(vec![vec![heading("Un")], vec![heading("Deux")]]));
    assert_eq!(doc.get_pages().len(), 2);
}

#[test]
fn it_sets_the_document_language_and_title() {
    let doc = load(&data(vec![vec![heading("Un")]]));
    let cat = catalog(&doc);
    assert_eq!(cat.get(b"Lang").unwrap().as_str().unwrap(), b"fr-CA");
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let title = doc.get_dictionary(info).unwrap().get(b"Title").unwrap().as_str().unwrap().to_vec();
    // A text string is UTF-16BE after a byte order mark, else PDFDocEncoding (ASCII here).
    let text = match title.strip_prefix(&[0xfe, 0xff]) {
        Some(utf16) => String::from_utf16(&utf16.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()).unwrap(),
        None => String::from_utf8(title).unwrap(),
    };
    assert_eq!(text, "Revue de conception");
}

/// The structure types (`/S`) of every element under the structure tree root.
fn structure_types(doc: &Document) -> Vec<String> {
    fn walk(doc: &Document, object: &Object, out: &mut Vec<String>) {
        match object {
            Object::Reference(r) => {
                if let Ok(o) = doc.get_object(*r) {
                    walk(doc, o, out);
                }
            }
            Object::Array(items) => items.iter().for_each(|i| walk(doc, i, out)),
            Object::Dictionary(d) => {
                if let Ok(name) = d.get(b"S").and_then(Object::as_name) {
                    out.push(String::from_utf8_lossy(name).into_owned());
                }
                if let Ok(kids) = d.get(b"K") {
                    walk(doc, kids, out);
                }
            }
            _ => {}
        }
    }
    let root = catalog(doc).get(b"StructTreeRoot").unwrap();
    let mut out = vec![];
    walk(doc, root, &mut out);
    out
}

fn table(head: &[&str], rows: &[&[&str]]) -> Block {
    Block::Table {
        caption: "Rues".into(),
        head: head.iter().map(|s| s.to_string()).collect(),
        rows: rows.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect(),
    }
}

#[test]
fn it_is_tagged() {
    let blocks = vec![heading("Un"), Block::Heading { level: 2, text: "Deux".into() }, para("Texte"), table(&["Nom", "Largeur"], &[&["Parc", "18"]])];
    let doc = load(&data(vec![blocks]));
    let cat = catalog(&doc);
    assert!(cat.has(b"StructTreeRoot"));
    let mark = cat.get(b"MarkInfo").unwrap();
    let mark = match mark {
        Object::Reference(r) => doc.get_dictionary(*r).unwrap(),
        other => other.as_dict().unwrap(),
    };
    assert_eq!(mark.get(b"Marked").unwrap().as_bool().unwrap(), true);
    let types = structure_types(&doc);
    for wanted in ["H1", "H2", "P", "Table", "TR", "TH", "TD"] {
        assert!(types.iter().any(|t| t == wanted), "{wanted} missing from {types:?}");
    }
}

#[test]
fn it_keeps_an_empty_cell_as_a_cell() {
    let doc = load(&data(vec![vec![table(&["A", "B", "C"], &[&["x", "", "z"]])]]));
    let types = structure_types(&doc);
    assert_eq!(types.iter().filter(|t| *t == "TH").count(), 3, "{types:?}");
    assert_eq!(types.iter().filter(|t| *t == "TD").count(), 3, "{types:?}");
    assert_eq!(types.iter().filter(|t| *t == "TR").count(), 2, "{types:?}");
}

#[test]
fn it_draws_the_text_of_every_block() {
    let doc = load(&data(vec![vec![heading("Réseau cyclable"), para("Voir « la rue »\u{a0}: 3\u{a0}m")]]));
    let text = page_text(&doc, 1);
    assert!(text.contains("Réseau cyclable"), "{text:?}");
    assert!(text.contains("Voir « la rue »\u{a0}: 3\u{a0}m"), "{text:?}");
}

/// The width of `text` in points at `size`, from the font's own advances (a missing glyph is .notdef, glyph 0).
fn measured(font: &[u8], text: &str, size: f32) -> f32 {
    let face = ttf_parser::Face::parse(font, 0).unwrap();
    let per_em = f32::from(face.units_per_em());
    text.chars().map(|c| f32::from(face.glyph_hor_advance(face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0))).unwrap()) * size / per_em).sum()
}

/// Checks that the page's lines start at the left margin and end inside the right one; returns how many there are.
fn assert_lines_inside(doc: &Document, size: f32) -> usize {
    let starts = text_starts(doc, 1);
    let text = page_text(doc, 1);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), starts.len(), "{lines:?} / {starts:?}");
    for (line, (x, _)) in lines.iter().zip(&starts) {
        assert!((x - 54.0).abs() < 0.5, "{line:?} starts at {x}");
        let end = x + measured(REGULAR, line.trim_end(), size);
        assert!(end <= 612.0 - 54.0, "{line:?} ends at {end}");
    }
    lines.len()
}

#[test]
fn it_wraps_a_long_paragraph_inside_the_margins() {
    let long = "mot ".repeat(100);
    let doc = load(&data(vec![vec![para(long.trim())]]));
    assert!(assert_lines_inside(&doc, 11.0) > 3);
}

#[test]
fn it_wraps_one_long_unbroken_name() {
    let name = "x".repeat(200);
    let doc = load(&data(vec![vec![para(&name)]]));
    assert!(assert_lines_inside(&doc, 11.0) > 1);
    // Every piece is drawn, none is lost.
    assert_eq!(page_text(&doc, 1).matches('x').count(), 200);
}

#[test]
fn it_draws_a_question_mark_for_a_character_the_font_lacks() {
    let face = ttf_parser::Face::parse(REGULAR, 0).unwrap();
    assert!(face.glyph_index('中').is_none(), "the font has U+4E2D, pick another");
    // PDF/UA-1 refuses .notdef, so the file is still written, with "?" where the font has no glyph.
    let doc = load(&data(vec![vec![para("Rue 中 Est")]]));
    assert!(page_text(&doc, 1).contains("Rue ? Est"));
    // And it wraps by the width of the "?" that is drawn.
    let long = "中".repeat(200);
    let starts = text_starts(&load(&data(vec![vec![para(&long)]])), 1);
    let per_line = ((504.0 / measured(REGULAR, "?", 11.0)) as usize).max(1);
    assert_eq!(starts.len(), 200_usize.div_ceil(per_line));
}

#[test]
fn the_fonts_are_static_and_hold_the_french_glyphs() {
    for font in [REGULAR, BOLD] {
        let face = ttf_parser::Face::parse(font, 0).unwrap();
        assert!(!face.is_variable());
        assert!(face.raw_face().table(ttf_parser::Tag::from_bytes(b"fvar")).is_none());
        for c in ['é', 'è', 'à', 'ç', 'É', '«', '»', '\u{a0}', '\u{202f}'] {
            assert!(face.glyph_index(c).is_some(), "no glyph for U+{:04X}", c as u32);
        }
    }
}

#[test]
fn it_reads_a_table() {
    let table = Block::Table {
        caption: "Rues".into(),
        head: vec!["Nom".into(), "Largeur".into()],
        rows: vec![vec!["Avenue du Parc".into(), "18\u{a0}m".into()], vec!["Rue Rachel".into(), "15\u{a0}m".into()]],
    };
    let doc = load(&data(vec![vec![table]]));
    let text = page_text(&doc, 1);
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s:?} missing in {text:?}"));
    assert!(at("Rues") < at("Nom"));
    assert!(at("Nom") < at("Largeur"));
    assert!(at("Largeur") < at("Avenue du Parc"));
    assert!(at("Avenue du Parc") < at("18\u{a0}m"));
    assert!(at("18\u{a0}m") < at("Rue Rachel"));
}

#[test]
fn it_asks_the_viewer_to_show_the_title() {
    let doc = load(&data(vec![vec![heading("Un")]]));
    let prefs = catalog(&doc).get(b"ViewerPreferences").expect("viewer preferences");
    let prefs = match prefs {
        Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
        other => other.as_dict().unwrap(),
    };
    assert_eq!(prefs.get(b"DisplayDocTitle").unwrap().as_bool().unwrap(), true);
}
