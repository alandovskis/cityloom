//! The report's review pages, after the pages of the changed places: the network effects, the area rules, the limits
//! of the OpenStreetMap data, and the appendix. Pure: each takes what the checks found and words it in the interface
//! language. A page may run past one sheet; the writer continues it on the next.

use cityloom_report::Block;

use crate::map::network::{Check, NetworkFinding, NetworkReport, Place, Severity};
use crate::map::source_quality::{Kind as SourceKind, SourceReport};
use crate::map::vm::AreaStreet;
use crate::restriction::model::Restriction;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say_now;
use crate::shared::units::Units;

/// The most findings listed for one check or kind of source gap; the rest are counted.
pub const MAX_ROWS: usize = 10;
/// The most places named for one finding; the rest are counted.
pub const MAX_PLACE_NAMES: usize = 2;
/// The most streets listed as breaking an area's rule; the rest are counted.
pub const MAX_AREA_STREETS: usize = 20;

fn say(i18n: &I18n, key: &str) -> String {
    i18n.tr_now(key, &Args::new())
}

fn count(i18n: &I18n, key: &str, n: usize) -> String {
    i18n.tr_now(key, &Args::new().num("n", n as i64))
}

/// "none", "1 finding" or "N findings", by the messages the Network checks tab uses.
fn findings_count(i18n: &I18n, n: usize) -> String {
    match n {
        0 => say(i18n, "net-count-none"),
        1 => say(i18n, "net-count-one"),
        _ => count(i18n, "net-count", n),
    }
}

/// The names of the first `MAX_PLACE_NAMES` places, then how many more there were.
fn place_names(places: &[Place], i18n: &I18n, units: Units) -> String {
    let names: Vec<String> = places.iter().take(MAX_PLACE_NAMES).map(|p| say_now(i18n, units, &p.name)).collect();
    let more = places.len().saturating_sub(MAX_PLACE_NAMES);
    let names = names.join(", ");
    if more == 0 { names } else { i18n.tr_now("map-report-places-and-more", &Args::new().str("names", names).num("n", more as i64)) }
}

/// The network checks: a table of the six checks with Pass, Notice or Fail, then, for each that found something,
/// its findings.
pub fn network_page(report: &NetworkReport, i18n: &I18n, units: Units) -> Vec<Block> {
    let (pass, fail, notice) = (say(i18n, "map-report-pass"), say(i18n, "map-report-fail"), say(i18n, "map-report-notice"));
    let mut page = vec![Block::Heading { level: 2, text: say(i18n, "map-report-network") }];
    let n = report.findings.len();
    page.push(Block::Paragraph { text: if n == 0 { say(i18n, "net-lead-none") } else { count(i18n, "net-lead", n) } });
    if report.unreadable > 0 {
        page.push(Block::Paragraph { text: count(i18n, "net-unreadable", report.unreadable) });
    }
    page.push(Block::Paragraph { text: say(i18n, "net-note") });
    let of = |rule: Check| report.findings.iter().filter(move |f| f.rule == rule);
    let rows = Check::ALL
        .iter()
        .map(|&rule| {
            let result = if of(rule).any(|f| f.severity == Severity::Problem) {
                fail.clone()
            } else if of(rule).next().is_some() {
                notice.clone()
            } else {
                pass.clone()
            };
            vec![say(i18n, &format!("net-rule-{}", rule.id())), findings_count(i18n, of(rule).count()), result]
        })
        .collect();
    page.push(Block::Table {
        caption: say(i18n, "map-report-network-caption"),
        head: vec![say(i18n, "map-report-col-check"), say(i18n, "map-report-col-findings"), say(i18n, "map-report-col-result")],
        rows,
    });
    for rule in Check::ALL {
        let found: Vec<&NetworkFinding> = of(rule).collect();
        if found.is_empty() {
            continue;
        }
        let row = |f: &&NetworkFinding| {
            let severity = match f.severity {
                Severity::Problem => "net-severity-problem",
                Severity::Notice => "net-severity-notice",
            };
            vec![say(i18n, severity), say_now(i18n, units, &f.said), place_names(&f.places, i18n, units)]
        };
        page.push(Block::Table {
            caption: say(i18n, &format!("net-rule-{}", rule.id())),
            head: vec![say(i18n, "map-report-col-severity"), say(i18n, "map-report-col-finding"), say(i18n, "map-report-col-places")],
            rows: found.iter().take(MAX_ROWS).map(row).collect(),
        });
        if found.len() > MAX_ROWS {
            page.push(Block::Paragraph { text: count(i18n, "net-more-findings", found.len() - MAX_ROWS) });
        }
    }
    page
}

/// The area rules the person drew, each with the streets inside it and how many keep to its rule.
pub fn area_page(areas: &[(Restriction, Vec<AreaStreet>)], i18n: &I18n, units: Units) -> Vec<Block> {
    let mut page = vec![Block::Heading { level: 2, text: say(i18n, "map-report-areas") }];
    if areas.is_empty() {
        page.push(Block::Paragraph { text: say(i18n, "map-report-areas-none") });
        return page;
    }
    let (pass, fail, notice) = (say(i18n, "map-report-pass"), say(i18n, "map-report-fail"), say(i18n, "map-report-notice"));
    page.push(Block::Paragraph { text: say(i18n, "map-report-areas-intro") });
    let rows = areas
        .iter()
        .map(|(r, streets)| {
            let conflicts = streets.iter().filter(|s| !s.conflicts.is_empty()).count();
            let result = if conflicts > 0 {
                fail.clone()
            } else if streets.iter().any(|s| s.speed_unknown) {
                notice.clone()
            } else {
                pass.clone()
            };
            vec![say_now(i18n, units, &r.title()), say_now(i18n, units, &r.rule.said()), streets.len().to_string(), conflicts.to_string(), result]
        })
        .collect();
    page.push(Block::Table {
        caption: say(i18n, "map-report-areas-caption"),
        head: ["map-report-col-area", "map-report-col-rule", "map-report-col-inside", "map-report-col-conflicts", "map-report-col-result"]
            .iter()
            .map(|k| say(i18n, k))
            .collect(),
        rows,
    });
    let breaking: Vec<Vec<String>> = areas
        .iter()
        .flat_map(|(r, streets)| {
            streets.iter().filter(|s| !s.conflicts.is_empty()).map(move |s| {
                let why = s.conflicts.iter().map(|c| say_now(i18n, units, &c.said(s.rule))).collect::<Vec<_>>().join(" ");
                vec![say_now(i18n, units, &s.name), say_now(i18n, units, &r.title()), why]
            })
        })
        .collect();
    if !breaking.is_empty() {
        let more = breaking.len().saturating_sub(MAX_AREA_STREETS);
        page.push(Block::Table {
            caption: say(i18n, "map-report-areas-streets-caption"),
            head: ["map-report-col-street", "map-report-col-area", "map-report-col-why"].iter().map(|k| say(i18n, k)).collect(),
            rows: breaking.into_iter().take(MAX_AREA_STREETS).collect(),
        });
        if more > 0 {
            page.push(Block::Paragraph { text: count(i18n, "map-report-areas-more", more) });
        }
    }
    page
}

/// Where the OpenStreetMap data behind the city may be missing or wrong, by kind, each with its source and the
/// design reading it could mislead.
pub fn limits_page(report: &SourceReport, i18n: &I18n, units: Units) -> Vec<Block> {
    let mut page = vec![Block::Heading { level: 2, text: say(i18n, "map-report-limits") }];
    page.push(Block::Paragraph { text: say(i18n, "prov-note") });
    let n = report.findings.len();
    page.push(Block::Paragraph { text: if n == 0 { say(i18n, "prov-lead-none") } else { count(i18n, "prov-lead", n) } });
    for kind in SourceKind::ALL {
        let found: Vec<_> = report.findings.iter().filter(|f| f.kind == kind).collect();
        if found.is_empty() {
            continue;
        }
        let said = |s: &crate::shared::said::Said| say_now(i18n, units, s);
        let (source, impact) = (said(&found[0].source), said(&found[0].impact));
        // What a kind of gap rests on is mostly one thing: say it once, and only repeat it where findings differ.
        let alike = found.iter().all(|f| said(&f.source) == source && said(&f.impact) == impact);
        let caption = say(i18n, &format!("prov-kind-{}", kind.id()));
        if alike {
            page.push(Block::Table {
                caption,
                head: vec![say(i18n, "map-report-col-finding")],
                rows: found.iter().take(MAX_ROWS).map(|f| vec![said(&f.said)]).collect(),
            });
        } else {
            page.push(Block::Table {
                caption,
                head: ["map-report-col-finding", "map-report-col-source", "map-report-col-impact"].iter().map(|k| say(i18n, k)).collect(),
                rows: found.iter().take(MAX_ROWS).map(|f| vec![said(&f.said), said(&f.source), said(&f.impact)]).collect(),
            });
        }
        if found.len() > MAX_ROWS {
            page.push(Block::Paragraph { text: count(i18n, "prov-more-findings", found.len() - MAX_ROWS) });
        }
        if alike {
            for (label, text) in [("prov-label-source", source), ("prov-label-impact", impact)] {
                page.push(Block::Paragraph { text: i18n.tr_now("map-report-labelled", &Args::new().str("label", say(i18n, label)).str("text", text)) });
            }
        }
    }
    page
}

/// The words the report uses and what its checks rest on: a glossary, the status of the thresholds, the credits.
pub fn appendix(i18n: &I18n) -> Vec<Block> {
    let terms: [(&str, &str); 8] = [
        ("map-report-pass", "map-report-def-pass"),
        ("map-report-fail", "map-report-def-fail"),
        ("net-severity-problem", "map-report-def-problem"),
        ("map-report-notice", "map-report-def-notice"),
        ("map-report-term-before-after", "map-report-def-before-after"),
        ("map-report-term-network", "map-report-def-network"),
        ("map-report-term-area", "map-report-def-area"),
        ("map-report-term-placeholder", "map-report-def-placeholder"),
    ];
    vec![
        Block::Heading { level: 2, text: say(i18n, "map-report-appendix") },
        Block::Heading { level: 3, text: say(i18n, "map-report-appendix-thresholds") },
        Block::Paragraph { text: say(i18n, "map-report-placeholder-notice") },
        Block::Paragraph { text: say(i18n, "map-report-appendix-thresholds-status") },
        Block::Heading { level: 3, text: say(i18n, "map-report-appendix-glossary") },
        Block::Table {
            caption: say(i18n, "map-report-appendix-glossary-caption"),
            head: vec![say(i18n, "map-report-col-term"), say(i18n, "map-report-col-meaning")],
            rows: terms.iter().map(|(term, meaning)| vec![say(i18n, term), say(i18n, meaning)]).collect(),
        },
        Block::Heading { level: 3, text: say(i18n, "map-report-appendix-credits") },
        Block::Paragraph { text: say(i18n, "map-report-credits-osm") },
        Block::Paragraph { text: say(i18n, "map-report-credits-atlas") },
        Block::Paragraph { text: say(i18n, "map-report-credits-software") },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::network::{Check, NetworkFinding, NetworkReport, Place, PlaceKind, Severity};
    use crate::map::source_quality::{Kind as SourceKind, SourceFinding, SourceReport};
    use crate::map::vm::AreaStreet;
    use crate::restriction::model::{Conflict, Restriction, Rule};
    use crate::shared::i18n::Locale;
    use crate::shared::said::{Arg, Said};
    use crate::shared::units::Units;

    fn en() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn fr() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::FrCa)
    }

    fn place(uid: u32) -> Place {
        Place { kind: PlaceKind::Street, uid, name: Said::new("city-unnamed") }
    }

    fn net(rule: Check, severity: Severity, places: usize) -> NetworkFinding {
        NetworkFinding { rule, severity, said: Said::new("net-crossing-missing"), places: (0..places as u32).map(place).collect() }
    }

    fn texts(blocks: &[Block]) -> Vec<String> {
        blocks
            .iter()
            .flat_map(|b| match b {
                Block::Heading { text, .. } | Block::Paragraph { text } => vec![text.clone()],
                Block::Table { caption, head, rows } => std::iter::once(caption.clone()).chain(head.clone()).chain(rows.concat()).collect(),
                Block::Figures { .. } => vec![],
            })
            .collect()
    }

    fn tables(blocks: &[Block]) -> Vec<(&String, &Vec<String>, &Vec<Vec<String>>)> {
        blocks.iter().filter_map(|b| if let Block::Table { caption, head, rows } = b { Some((caption, head, rows)) } else { None }).collect()
    }

    #[test]
    fn a_network_with_nothing_found_says_so_and_every_check_passes() {
        let page = network_page(&NetworkReport::default(), &en(), Units::Metres);
        let all = texts(&page).join("\n");
        assert!(all.contains("Network effects") && all.contains("found nothing"), "{all}");
        let (_, _, rows) = tables(&page)[0];
        assert_eq!(rows.len(), Check::ALL.len(), "one row for each of the six checks");
        assert!(rows.iter().all(|r| r[1] == "no findings" && r[2] == "Pass"), "{rows:?}");
        assert_eq!(tables(&page).len(), 1, "no findings table when nothing was found");
    }

    #[test]
    fn a_problem_fails_its_check_a_notice_only_flags_it_and_the_rest_pass() {
        let report = NetworkReport { findings: vec![net(Check::Crossings, Severity::Problem, 1), net(Check::Cycling, Severity::Notice, 2)], unreadable: 3 };
        let page = network_page(&report, &en(), Units::Metres);
        let (_, _, rows) = tables(&page)[0];
        let row = |label: &str| rows.iter().find(|r| r[0] == label).unwrap().clone();
        assert_eq!(row("Crossings")[2], "Fail");
        assert_eq!(row("Cycling links")[2], "Notice");
        assert_eq!(row("Walking routes")[2], "Pass");
        let all = texts(&page).join("\n");
        assert!(all.contains("3 junctions could not be read"), "{all}");
        // One findings table for each check that found something, naming the severity in words.
        let found: Vec<_> = tables(&page).into_iter().skip(1).collect();
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|(c, _, r)| c.as_str() == "Crossings" && r[0][0] == "Problem"));
        assert!(found.iter().any(|(c, _, r)| c.as_str() == "Cycling links" && r[0][0] == "Notice"));
    }

    #[test]
    fn findings_and_places_are_capped_and_the_rest_counted() {
        let report =
            NetworkReport { findings: (0..MAX_ROWS + 4).map(|_| net(Check::Crossings, Severity::Problem, MAX_PLACE_NAMES + 2)).collect(), unreadable: 0 };
        let page = network_page(&report, &en(), Units::Metres);
        let found = tables(&page)[1];
        assert_eq!(found.2.len(), MAX_ROWS);
        assert!(found.2[0][2].contains("and 2 more places"), "{:?}", found.2[0]);
        assert!(texts(&page).iter().any(|t| t == "4 more findings are not listed."));
    }

    #[test]
    fn the_network_page_follows_the_language() {
        let report = NetworkReport { findings: vec![net(Check::Crossings, Severity::Problem, 1)], unreadable: 0 };
        let all = texts(&network_page(&report, &fr(), Units::Metres)).join("\n");
        assert!(all.contains("Effets sur le réseau") && all.contains("Échec") && !all.contains("Network effects"), "{all}");
    }

    fn area(id: &str, name: &str, rule: Rule) -> Restriction {
        Restriction { id: id.into(), name: name.into(), rule, ring: vec![] }
    }

    fn inside(area: &str, uid: u32, rule: Rule, conflicts: Vec<Conflict>, speed_unknown: bool) -> AreaStreet {
        AreaStreet { area: area.into(), uid, href: String::new(), name: Said::new("city-unnamed"), rule, conflicts, speed_unknown }
    }

    #[test]
    fn no_area_rules_is_said_in_one_line() {
        let all = texts(&area_page(&[], &en(), Units::Metres)).join("\n");
        assert!(all.contains("Area rules") && all.contains("No area rules"), "{all}");
    }

    #[test]
    fn each_area_gets_a_row_with_pass_notice_or_fail_and_its_conflicting_streets_are_listed() {
        let slow = Rule::SpeedLimit { kmh: 30 };
        let free = Rule::CarFree;
        let conflict = vec![Conflict::Pieces { kind: "travel", pieces: 2 }];
        let areas = vec![
            (area("r1", "Old town", free), vec![inside("r1", 1, free, conflict, false), inside("r1", 2, free, vec![], false)]),
            (area("r2", "", slow), vec![inside("r2", 3, slow, vec![], true)]),
            (area("r3", "Quiet", Rule::NoParkingOrLoading), vec![inside("r3", 4, Rule::NoParkingOrLoading, vec![], false)]),
        ];
        let page = area_page(&areas, &en(), Units::Metres);
        let (_, head, rows) = tables(&page)[0];
        assert_eq!(head.len(), 5);
        assert_eq!((rows[0][2].as_str(), rows[0][3].as_str(), rows[0][4].as_str()), ("2", "1", "Fail"));
        assert_eq!(rows[1][4], "Notice", "a street whose speed is unknown cannot be checked");
        assert_eq!(rows[2][4], "Pass");
        let (_, _, streets) = tables(&page)[1];
        assert_eq!(streets.len(), 1, "only the street that conflicts");
        assert!(streets[0][1].contains("Old town"), "{streets:?}");
    }

    #[test]
    fn the_source_data_page_lists_each_kind_with_its_source_and_design_impact() {
        let finding = |kind| SourceFinding {
            kind,
            said: Said::new("prov-ends").with("street", Arg::Text("Rue Laurier".into())),
            source: Said::new("prov-source-ends"),
            impact: Said::new("prov-impact-ends"),
            places: vec![place(1)],
        };
        let report = SourceReport { findings: (0..MAX_ROWS + 2).map(|_| finding(SourceKind::DeadEnd)).collect() };
        let page = limits_page(&report, &en(), Units::Metres);
        let all = texts(&page).join("\n");
        assert!(all.contains("Limits of the OpenStreetMap data") && all.contains("not design failures"), "{all}");
        let found = tables(&page);
        assert_eq!(found.len(), 1, "only the kinds that found something");
        assert_eq!(found[0].1, &vec!["Finding".to_string()], "findings that share a source and an impact state them once, below");
        assert_eq!(found[0].2.len(), MAX_ROWS);
        assert_eq!(texts(&page).iter().filter(|t| t.starts_with("Source: ")).count(), 1);
        assert_eq!(texts(&page).iter().filter(|t| t.starts_with("Design impact: ")).count(), 1);
        assert!(found[0].2[0][0].contains("Rue Laurier"));
        assert!(all.contains("2 more"), "{all}");
        // Findings of one kind whose source differs keep a Source and a Design impact column.
        let mut mixed = report.findings.clone();
        mixed[0].source = Said::new("prov-source-width");
        let page = limits_page(&SourceReport { findings: mixed }, &en(), Units::Metres);
        assert_eq!(tables(&page)[0].1.len(), 3);
        let none = texts(&limits_page(&SourceReport::default(), &en(), Units::Metres)).join("\n");
        assert!(none.contains("Nothing looks missing"), "{none}");
    }

    #[test]
    fn the_appendix_defines_the_words_the_report_uses_and_credits_its_sources_in_both_languages() {
        for i18n in [en(), fr()] {
            let page = appendix(&i18n);
            let (_, head, rows) = tables(&page)[0];
            assert_eq!(head.len(), 2);
            let terms: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
            for word in [i18n.tr_now("map-report-pass", &Default::default()), i18n.tr_now("map-report-fail", &Default::default())] {
                assert!(terms.contains(&word.as_str()), "{terms:?}");
            }
            assert!(rows.iter().all(|r| !r[1].is_empty()));
            let all = texts(&page).join("\n");
            assert!(all.contains("OpenStreetMap") && all.contains("Noto Sans"), "{all}");
        }
    }
}
