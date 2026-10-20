mod support;

use std::fs;
use std::sync::{Arc, Mutex};

use cityloom_prepare::download::{self, get};
use support::{Reply, Request, serve};

fn body(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// Answers like a server that honours `Range`, and records what it was asked.
fn ranged(whole: Vec<u8>, seen: Arc<Mutex<Vec<(Option<u64>, Option<String>)>>>) -> impl Fn(&Request) -> Reply + Send + Sync + 'static {
    move |request| {
        seen.lock().unwrap().push((request.range_from, request.accept_encoding.clone()));
        match request.range_from {
            Some(from) => {
                let rest = whole[from as usize..].to_vec();
                Reply::Body { status: 206, content_length: Some(rest.len()), body: rest }
            }
            None => Reply::Body { status: 200, content_length: Some(whole.len()), body: whole.clone() },
        }
    }
}

#[test]
fn a_fresh_download_writes_the_file_and_no_part() {
    let whole = body(3_000_000);
    let (url, _) = serve(ranged(whole.clone(), Default::default()));
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("sub/file.bin");
    let mut said = Vec::new();
    download::download(&format!("{url}/f"), &target, true, &mut |done, total| said.push((done, total))).unwrap();
    assert_eq!(fs::read(&target).unwrap(), whole);
    assert!(!dir.path().join("sub/file.bin.part").exists());
    assert!(said.len() >= 3, "{said:?}");
    assert!(said.windows(2).all(|w| w[0].0 < w[1].0));
    assert!(said.iter().all(|s| s.1 == Some(3_000_000)));
    assert_eq!(said.last().unwrap().0, 3_000_000);
}

#[test]
fn the_request_asks_for_the_bytes_as_they_are() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (url, _) = serve(ranged(body(10), seen.clone()));
    let dir = tempfile::tempdir().unwrap();
    download::download(&format!("{url}/f"), &dir.path().join("f"), true, &mut |_, _| {}).unwrap();
    assert_eq!(seen.lock().unwrap().as_slice(), [(None, Some("identity".to_string()))]);
}

#[test]
fn a_part_file_is_continued_when_the_server_honours_range() {
    let whole = body(5000);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (url, _) = serve(ranged(whole.clone(), seen.clone()));
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("f");
    fs::write(dir.path().join("f.part"), &whole[..1000]).unwrap();
    let mut last = None;
    download::download(&format!("{url}/f"), &target, true, &mut |done, total| last = Some((done, total))).unwrap();
    assert_eq!(fs::read(&target).unwrap(), whole);
    assert_eq!(seen.lock().unwrap()[0].0, Some(1000));
    // the total counts what was in hand
    assert_eq!(last, Some((5000, Some(5000))));
}

#[test]
fn a_server_that_ignores_range_restarts_from_zero() {
    let whole = body(5000);
    let expected = whole.clone();
    let (url, _) = serve(move |_| Reply::Body { status: 200, content_length: Some(whole.len()), body: whole.clone() });
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("f");
    fs::write(dir.path().join("f.part"), b"junkjunk").unwrap();
    download::download(&format!("{url}/f"), &target, true, &mut |_, _| {}).unwrap();
    assert_eq!(fs::read(&target).unwrap(), expected);
}

#[test]
fn a_416_with_bytes_in_hand_means_the_file_was_whole() {
    let whole = body(5000);
    let (url, _) = serve(|_| Reply::Status(416));
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("f");
    fs::write(dir.path().join("f.part"), &whole).unwrap();
    download::download(&format!("{url}/f"), &target, true, &mut |_, _| {}).unwrap();
    assert_eq!(fs::read(&target).unwrap(), whole);
    assert!(!dir.path().join("f.part").exists());
}

#[test]
fn a_416_with_nothing_in_hand_is_an_error() {
    let (url, _) = serve(|_| Reply::Status(416));
    let dir = tempfile::tempdir().unwrap();
    let error = download::download(&format!("{url}/f"), &dir.path().join("f"), true, &mut |_, _| {}).unwrap_err();
    assert_eq!(error.lines, [format!("{url}/f: HTTP 416")]);
    assert!(!dir.path().join("f.part").exists());
}

#[test]
fn a_truncated_answer_is_an_error_that_keeps_the_part() {
    let (url, _) = serve(|_| Reply::Body { status: 200, body: body(600), content_length: Some(1000) });
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("f");
    let error = download::download(&format!("{url}/f"), &target, true, &mut |_, _| {}).unwrap_err();
    assert_eq!(error.lines, [format!("{url}/f: got 600 bytes of 1000; run again to continue")]);
    assert_eq!(fs::read(dir.path().join("f.part")).unwrap(), body(600));
    assert!(!target.exists());
}

#[test]
fn a_truncated_resumed_answer_counts_what_was_in_hand() {
    // 1000 in hand, 4000 promised, 1500 sent: 2500 of 5000.
    let (url, _) = serve(|_| Reply::Body { status: 206, body: body(1500), content_length: Some(4000) });
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("f.part"), body(1000)).unwrap();
    let error = download::download(&format!("{url}/f"), &dir.path().join("f"), true, &mut |_, _| {}).unwrap_err();
    assert_eq!(error.lines, [format!("{url}/f: got 2500 bytes of 5000; run again to continue")]);
    assert_eq!(fs::metadata(dir.path().join("f.part")).unwrap().len(), 2500);
}

#[test]
fn another_status_is_an_error() {
    let (url, _) = serve(|_| Reply::Status(404));
    let dir = tempfile::tempdir().unwrap();
    let error = download::download(&format!("{url}/f"), &dir.path().join("f"), true, &mut |_, _| {}).unwrap_err();
    assert_eq!(error.lines, [format!("{url}/f: HTTP 404")]);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn without_resume_a_failed_download_leaves_no_part() {
    let (url, _) = serve(|_| Reply::Body { status: 200, body: body(600), content_length: Some(1000) });
    let dir = tempfile::tempdir().unwrap();
    let error = download::download(&format!("{url}/f"), &dir.path().join("f"), false, &mut |_, _| {}).unwrap_err();
    assert!(error.lines[0].contains("got 600 bytes of 1000"), "{error}");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn without_resume_an_old_part_is_not_continued() {
    let whole = body(500);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (url, _) = serve(ranged(whole.clone(), seen.clone()));
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("f.part"), b"old").unwrap();
    download::download(&format!("{url}/f"), &dir.path().join("f"), false, &mut |_, _| {}).unwrap();
    assert_eq!(seen.lock().unwrap()[0].0, None);
    assert_eq!(fs::read(dir.path().join("f")).unwrap(), whole);
}

#[test]
fn redirects_are_followed_and_the_release_one_is_remembered() {
    let (url, _) = serve(|request| match request.path.as_str() {
        "/latest" => Reply::Redirect("/x/releases/download/v1/p.jar".into()),
        "/x/releases/download/v1/p.jar" => Reply::Redirect(format!("http://{}/signed?token=1", request.host)),
        "/signed?token=1" => Reply::Body { status: 200, body: b"jar".to_vec(), content_length: Some(3) },
        _ => Reply::Status(404),
    });
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("p.jar");
    let release = download::download(&format!("{url}/latest"), &target, false, &mut |_, _| {}).unwrap();
    assert_eq!(release, Some(format!("{url}/x/releases/download/v1/p.jar")));
    assert_eq!(fs::read(&target).unwrap(), b"jar");
}

#[test]
fn a_relative_location_is_resolved_against_the_address_that_sent_it() {
    let (url, _) = serve(|request| match request.path.as_str() {
        "/a/b/start?q=1" => Reply::Redirect("next".into()),
        "/a/b/next" => Reply::Redirect("../up?x=2".into()),
        "/a/up?x=2" => Reply::Redirect("/root".into()),
        "/root" => Reply::Redirect("//".to_string() + &request.host + "/net"),
        "/net" => Reply::Body { status: 200, body: b"ok".to_vec(), content_length: Some(2) },
        _ => Reply::Status(404),
    });
    let mut fetched = get(&format!("{url}/a/b/start?q=1#frag"), 0).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut fetched.body, &mut text).unwrap();
    assert_eq!((fetched.status, fetched.content_length, text.as_str(), fetched.release), (200, Some(2), "ok", None));
}

#[test]
fn a_redirect_keeps_the_range() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let (url, _) = serve(move |request| {
        log.lock().unwrap().push((request.path.clone(), request.range_from));
        match request.path.as_str() {
            "/a" => Reply::Redirect("/b".into()),
            _ => Reply::Body { status: 206, body: b"rest".to_vec(), content_length: Some(4) },
        }
    });
    let fetched = get(&format!("{url}/a"), 7).unwrap();
    assert_eq!(fetched.status, 206);
    assert_eq!(seen.lock().unwrap().as_slice(), [("/a".to_string(), Some(7)), ("/b".to_string(), Some(7))]);
}

#[test]
fn more_than_ten_redirects_is_an_error() {
    let (url, _) = serve(|request| {
        let n: u32 = request.path.trim_start_matches('/').parse().unwrap_or(0);
        if n < 11 { Reply::Redirect(format!("/{}", n + 1)) } else { Reply::Body { status: 200, body: vec![], content_length: Some(0) } }
    });
    let error = get(&format!("{url}/0"), 0).err().unwrap();
    assert_eq!(error.lines, [format!("{url}/0: more than 10 redirects")]);
}

#[test]
fn ten_redirects_are_followed() {
    let (url, _) = serve(|request| {
        let n: u32 = request.path.trim_start_matches('/').parse().unwrap_or(0);
        if n < 10 { Reply::Redirect(format!("/{}", n + 1)) } else { Reply::Body { status: 200, body: vec![], content_length: Some(0) } }
    });
    assert_eq!(get(&format!("{url}/0"), 0).unwrap().status, 200);
}

#[test]
fn a_redirect_with_no_location_is_an_error() {
    let (url, _) = serve(|_| Reply::Status(302));
    let error = get(&format!("{url}/a"), 0).err().unwrap();
    assert_eq!(error.lines, [format!("{url}/a: a redirect with no Location")]);
}
