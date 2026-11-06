//! Reads the written files back with lopdf.

use cityloom_report::{Block, ReportData, write_pdf};
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

#[test]
fn it_is_tagged() {
    let doc = load(&data(vec![vec![heading("Un"), para("Texte")]]));
    let cat = catalog(&doc);
    assert!(cat.has(b"StructTreeRoot"));
    let mark = cat.get(b"MarkInfo").unwrap();
    let mark = match mark {
        Object::Reference(r) => doc.get_dictionary(*r).unwrap(),
        other => other.as_dict().unwrap(),
    };
    assert_eq!(mark.get(b"Marked").unwrap().as_bool().unwrap(), true);
}

#[test]
fn it_draws_the_text_of_every_block() {
    let doc = load(&data(vec![vec![heading("Réseau cyclable"), para("Voir « la rue »\u{a0}: 3\u{a0}m")]]));
    let text = page_text(&doc, 1);
    assert!(text.contains("Réseau cyclable"), "{text:?}");
    assert!(text.contains("Voir « la rue »\u{a0}: 3\u{a0}m"), "{text:?}");
}

#[test]
fn it_wraps_a_long_paragraph_inside_the_margins() {
    let long = "mot ".repeat(100);
    let doc = load(&data(vec![vec![para(long.trim())]]));
    let starts = text_starts(&doc, 1);
    assert!(starts.len() > 3, "{starts:?}");
    assert!(starts.iter().all(|(x, _)| (*x - 54.0).abs() < 0.5), "{starts:?}");
}

#[test]
fn it_wraps_one_long_unbroken_name() {
    let name = "x".repeat(200);
    let doc = load(&data(vec![vec![para(&name)]]));
    let starts = text_starts(&doc, 1);
    assert!(starts.len() > 1, "{starts:?}");
    assert!(starts.iter().all(|(x, _)| (*x - 54.0).abs() < 0.5), "{starts:?}");
    // Every piece is drawn, none is lost.
    assert_eq!(page_text(&doc, 1).matches('x').count(), 200);
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
