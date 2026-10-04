use cityloom_editor::city::model::City;
use osm_import::import;
use std::time::Instant;
#[test]
fn montreal() {
    let mut files: Vec<_> = std::fs::read_dir("/tmp/mtl").unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for f in files {
        let osm = std::fs::read(&f).unwrap();
        let t = Instant::now();
        let net = match std::panic::catch_unwind(|| import(&osm)) {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                println!("{:?}: IMPORT ERROR {e}", f.file_name().unwrap());
                continue;
            }
            Err(_) => {
                println!("{:?}: IMPORT PANIC", f.file_name().unwrap());
                continue;
            }
        };
        let ti = t.elapsed();
        let t = Instant::now();
        let city = std::panic::catch_unwind(|| City::from_network(&net, "x"));
        let Ok(city) = city else {
            println!("{:?}: CITY PANIC", f.file_name().unwrap());
            continue;
        };
        let v = city.view(0);
        let tc = t.elapsed();
        let mut deg = std::collections::HashMap::new();
        for r in &net.roads {
            *deg.entry(r.from).or_insert(0) += 1;
            *deg.entry(r.to).or_insert(0) += 1;
        }
        let cand = net.nodes.iter().filter(|n| n.junction && (3..=5).contains(&deg.get(&n.id).copied().unwrap_or(0))).count();
        let over5 = net.nodes.iter().filter(|n| n.junction && deg.get(&n.id).copied().unwrap_or(0) > 5).count();
        let mut hist = std::collections::BTreeMap::new();
        for n in &net.nodes {
            *hist.entry(deg.get(&n.id).copied().unwrap_or(0)).or_insert(0) += 1;
        }
        println!("   osm junctions with 3-5 roads: {cand}; >5 roads: {over5}; node degree histogram {hist:?}");
        let (mut jbad, mut sbad) = (0, 0);
        for n in v.nodes.iter().filter(|n| n.junction) {
            if city.junction_editor(n.uid, 0).is_none() {
                jbad += 1
            }
        }
        for e in &v.edges {
            if city.street_editor(e.uid, 0).is_none() {
                sbad += 1
            }
        }
        let degree_nonjunction = v.nodes.iter().filter(|n| !n.junction).count();
        let max_row = v.edges.iter().map(|e| e.row_mm).max().unwrap_or(0);
        let tiny = v.edges.iter().filter(|e| e.length_mm < 3000).count();
        println!(
            "{:?}: roads {} nodes {} | junctions {} (unopenable {jbad}) streets {} (unopenable {sbad}) nonjunction {} | failing {} | max row {} m, streets<3m long {} | import {:?} city {:?}",
            f.file_name().unwrap(),
            net.roads.len(),
            net.nodes.len(),
            v.nodes.iter().filter(|n| n.junction).count(),
            v.edges.len(),
            degree_nonjunction,
            v.failing,
            max_row / 1000,
            tiny,
            ti,
            tc
        );
    }
}
