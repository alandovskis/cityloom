//! Reads the written files back with lopdf.

use cityloom_report::{Block, Captioned, Figure, Item, ReportData, Rgb};
use cityloom_report_pdf::write_pdf;
use lopdf::{Document, Object};

const REGULAR: &[u8] = include_bytes!("../../../web/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../web/fonts/NotoSans-Bold.ttf");

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

fn captioned(name: &str, w: f32, h: f32, items: Vec<Item>) -> Captioned {
    Captioned {
        caption: format!("Légende {name}"),
        alt: format!("Texte alternatif {name}"),
        note: format!("Note {name}"),
        figure: Figure { width: w, height: h, items, omitted: 0 },
    }
}

fn square(w: f32, h: f32) -> Vec<Item> {
    vec![
        Item::Poly { pts: vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], fill: Rgb(200, 200, 200), stroke: Some((Rgb(0, 0, 0), 1.0)) },
        Item::Line { pts: vec![(0.0, 0.0), (w, h)], color: Rgb(255, 0, 0), width: 1.0 },
        Item::Text { at: (2.0, h / 2.0), size: 10.0, color: Rgb(0, 0, 0), text: "Rue".into() },
    ]
}

fn two() -> Block {
    Block::Figures { items: vec![captioned("A", 100.0, 50.0, square(100.0, 50.0)), captioned("B", 60.0, 120.0, square(60.0, 120.0))] }
}

/// The operations of the page's content stream.
fn operations(doc: &Document, n: u32) -> Vec<lopdf::content::Operation> {
    let id = *doc.get_pages().get(&n).unwrap();
    doc.get_and_decode_page_content(id).unwrap().operations
}

fn number(o: &Object) -> f32 {
    match o {
        Object::Integer(v) => *v as f32,
        Object::Real(v) => *v,
        _ => f32::NAN,
    }
}

/// Every path point (x, y) the page draws (`m`, `l`, `c`, and both corners of `re`), as written in the stream.
fn path_points(doc: &Document, n: u32) -> Vec<(f32, f32)> {
    let mut pts = vec![];
    for op in operations(doc, n) {
        let v: Vec<f32> = op.operands.iter().map(number).collect();
        match op.operator.as_str() {
            "m" | "l" => pts.push((v[0], v[1])),
            "c" => pts.extend(v.chunks(2).map(|c| (c[0], c[1]))),
            "re" => {
                pts.push((v[0], v[1]));
                pts.push((v[0] + v[2], v[1] + v[3]));
            }
            _ => {}
        }
    }
    pts
}

fn figure_alts(doc: &Document) -> Vec<String> {
    fn walk(doc: &Document, object: &Object, out: &mut Vec<String>) {
        match object {
            Object::Reference(r) => {
                if let Ok(o) = doc.get_object(*r) {
                    walk(doc, o, out);
                }
            }
            Object::Array(items) => items.iter().for_each(|i| walk(doc, i, out)),
            Object::Dictionary(d) => {
                if d.get(b"S").and_then(Object::as_name).is_ok_and(|n| n == b"Figure") {
                    let alt = d.get(b"Alt").expect("a figure has /Alt").as_str().unwrap();
                    out.push(match alt.strip_prefix(&[0xfe, 0xff]) {
                        Some(u) => String::from_utf16(&u.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()).unwrap(),
                        None => String::from_utf8(alt.to_vec()).unwrap(),
                    });
                }
                if let Ok(kids) = d.get(b"K") {
                    walk(doc, kids, out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    walk(doc, catalog(doc).get(b"StructTreeRoot").unwrap(), &mut out);
    out
}

#[test]
fn a_figures_block_draws_inside_the_margins() {
    let doc = load(&data(vec![vec![heading("Un"), two()]]));
    let pts = path_points(&doc, 1);
    assert!(pts.len() > 8, "{pts:?}");
    for (x, y) in pts {
        assert!((54.0 - 0.01..=558.0 + 0.01).contains(&x), "x {x}");
        assert!((54.0 - 0.01..=738.0 + 0.01).contains(&y), "y {y}");
    }
    for (x, y) in text_starts(&doc, 1) {
        assert!((54.0 - 0.01..=558.0).contains(&x) && (0.0..=738.0).contains(&y), "text at {x},{y}");
    }
}

#[test]
fn each_figure_is_a_tagged_figure_with_its_alt_text() {
    let doc = load(&data(vec![vec![two()]]));
    assert_eq!(figure_alts(&doc), ["Texte alternatif A", "Texte alternatif B"]);
}

#[test]
fn captions_and_notes_are_in_the_page_text() {
    let doc = load(&data(vec![vec![two()]]));
    let text = page_text(&doc, 1);
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s:?} missing in {text:?}"));
    assert!(at("Légende A") < at("Note A"));
    assert!(at("Légende B") < at("Note B"));
    assert!(text.contains("Rue"), "{text:?}");
    let types = structure_types(&doc);
    assert!(types.iter().filter(|t| *t == "P").count() >= 4, "{types:?}");
}

#[test]
fn one_figure_alone_stays_in_its_half() {
    let doc = load(&data(vec![vec![Block::Figures { items: vec![captioned("A", 100.0, 50.0, square(100.0, 50.0))] }]]));
    for (x, _) in path_points(&doc, 1) {
        assert!((54.0 - 0.01..=54.0 + 246.0 + 0.01).contains(&x), "x {x}");
    }
    assert_eq!(figure_alts(&doc).len(), 1);
}

#[test]
fn a_figure_with_no_items_still_draws_its_caption() {
    let doc = load(&data(vec![vec![Block::Figures { items: vec![captioned("A", 100.0, 50.0, vec![])] }]]));
    let text = page_text(&doc, 1);
    assert!(text.contains("Légende A") && text.contains("Note A"), "{text:?}");
}

#[test]
fn the_document_still_validates_as_ua1() {
    // write_pdf validates against PDF/UA-1 and errors otherwise; a missing glyph in a figure's text is drawn as "?".
    let mut items = square(100.0, 50.0);
    items.push(Item::Text { at: (5.0, 5.0), size: 8.0, color: Rgb(1, 2, 3), text: "中".into() });
    let block = Block::Figures { items: vec![captioned("A", 100.0, 50.0, items), captioned("B", 100.0, 50.0, vec![])] };
    let doc = load(&data(vec![vec![heading("Un"), para("Texte"), block]]));
    assert!(page_text(&doc, 1).contains('?'));
}

#[test]
fn a_figures_block_taller_than_the_page_is_scaled_to_fit() {
    let block = Block::Figures { items: vec![captioned("A", 10.0, 5000.0, square(10.0, 5000.0)), captioned("B", 10.0, 50.0, square(10.0, 50.0))] };
    let doc = load(&data(vec![vec![heading("Un"), block]]));
    assert_eq!(doc.get_pages().len(), 1);
    for (x, y) in path_points(&doc, 1) {
        assert!((54.0 - 0.01..=558.0 + 0.01).contains(&x) && (54.0 - 0.01..=738.0 + 0.01).contains(&y), "{x},{y}");
    }
    let text = page_text(&doc, 1);
    assert!(text.contains("Note A") && text.contains("Note B"), "{text:?}");
}

#[test]
fn stroke_widths_scale_with_the_fit() {
    // 100 wide fitted into a 246 pt half is scaled 2.46, so a width of 1 is drawn 2.46; a hairline is kept at 0.25.
    let thin = vec![Item::Line { pts: vec![(0.0, 0.0), (100.0, 50.0)], color: Rgb(0, 0, 0), width: 0.01 }];
    let thick = vec![Item::Line { pts: vec![(0.0, 0.0), (100.0, 50.0)], color: Rgb(0, 0, 0), width: 1.0 }];
    let widths = |items: Vec<Item>| -> Vec<f32> {
        let doc = load(&data(vec![vec![Block::Figures { items: vec![captioned("A", 100.0, 50.0, items)] }]]));
        operations(&doc, 1).iter().filter(|o| o.operator == "w").map(|o| number(&o.operands[0])).collect()
    };
    let w = widths(thick);
    assert!(w.iter().any(|v| (v - 2.46).abs() < 0.01), "{w:?}");
    let w = widths(thin);
    assert!(w.iter().any(|v| (v - 0.25).abs() < 0.01), "{w:?}");
}

#[test]
fn a_table_longer_than_a_page_continues_on_the_next_with_its_head_repeated() {
    let rows: Vec<Vec<String>> = (0..70).map(|i| vec![format!("Rangée{i:02}"), "Réussite".into()]).collect();
    let block = Block::Table { caption: "Rues".into(), head: vec!["Nom".into(), "Résultat".into()], rows };
    let doc = load(&data(vec![vec![heading("Un"), block]]));
    let pages = doc.get_pages().len() as u32;
    assert!(pages >= 2, "{pages} pages");
    let all: String = (1..=pages).map(|n| page_text(&doc, n)).collect();
    for i in 0..70 {
        assert!(all.contains(&format!("Rangée{i:02}")), "row {i} is missing");
    }
    assert!(page_text(&doc, 2).contains("Résultat"), "the head is repeated");
}

#[test]
fn a_long_paragraph_continues_on_the_next_page() {
    let words: Vec<String> = (0..1500).map(|i| format!("mot{i}")).collect();
    let doc = load(&data(vec![vec![heading("Un"), para(&words.join(" "))]]));
    let pages = doc.get_pages().len() as u32;
    assert!(pages >= 2);
    let all: String = (1..=pages).map(|n| page_text(&doc, n)).collect();
    assert!(all.contains("mot1499") && all.contains("mot0"));
}

#[test]
fn a_table_that_does_not_fit_what_is_left_splits_between_rows_and_loses_none() {
    let rows: Vec<Vec<String>> = (0..3).map(|i| vec![format!("Rangée{i}"), "x".into()]).collect();
    let table = Block::Table { caption: "Rues".into(), head: vec!["Nom".into(), "R".into()], rows };
    let filler = para(&vec!["ligne"; 700].join(" "));
    let doc = load(&data(vec![vec![filler, table]]));
    let pages = doc.get_pages().len() as u32;
    let all: String = (1..=pages).map(|n| page_text(&doc, n)).collect();
    assert!(pages >= 2 && all.contains("Rues") && all.contains("Rangée0") && all.contains("Rangée2"), "{all}");
    assert!(page_text(&doc, pages).contains("Nom"), "the head leads the continuation");
}

#[test]
fn content_that_cannot_fit_a_page_is_an_error_and_never_dropped_silently() {
    let tall = vec!["ligne".to_string(); 200].join("\n");
    let block = Block::Table { caption: String::new(), head: vec!["A".into()], rows: vec![vec![tall]] };
    let err = write_pdf(&data(vec![vec![block]]), REGULAR, BOLD).expect_err("a row taller than a page");
    assert!(err.contains("cut"), "{err}");
}
