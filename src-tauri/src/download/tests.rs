//! Tests für Index, Skip-Entscheidung und einzelne Downloads gegen den lokalen Testserver.

use super::fetch::{run_task, Done, Opts, Task};
use super::index::{decide_skip, index_key, is_unchanged, load_index, save_index, Index, IndexEntry};
use super::preview::{classify, Class};
use super::progress::Shared;
use super::sink::{finalize, part_path, zip_method, Sink};
use crate::session::Ctx;
use crate::testutil::{resp, spawn, truncated};
use crate::types::DownloadItem;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Url;
use zip::{CompressionMethod, ZipWriter};

fn entry(size: u64, etag: Option<&str>, modified: Option<&str>) -> IndexEntry {
    IndexEntry {
        size,
        etag: etag.map(Into::into),
        modified: modified.map(Into::into),
        url: "u".into(),
    }
}

#[test]
fn unchanged_prefers_etag() {
    let e = entry(10, Some("\"a\""), Some("Mon"));
    assert!(is_unchanged(&e, Some("\"a\""), Some("Tue"), Some(99)));
    assert!(!is_unchanged(&e, Some("\"b\""), Some("Mon"), Some(10)));
}

#[test]
fn unchanged_falls_back_to_modified_then_size() {
    let e = entry(10, None, Some("Mon"));
    assert!(is_unchanged(&e, None, Some("Mon"), Some(10)));
    assert!(!is_unchanged(&e, None, Some("Tue"), Some(10)));
    assert!(!is_unchanged(&e, None, Some("Mon"), Some(11)));
    let e2 = entry(10, None, None);
    assert!(is_unchanged(&e2, None, None, Some(10)));
    assert!(!is_unchanged(&e2, None, None, None));
}

#[test]
fn skip_decision() {
    let e = entry(10, None, Some("Mon"));
    // inkrementell aus: nie überspringen
    assert!(!decide_skip(
        false,
        false,
        Some(&e),
        Some(10),
        None,
        Some("Mon"),
        Some(10)
    ));
    // Ordner: Datei fehlt lokal, also neu laden
    assert!(!decide_skip(true, false, Some(&e), None, None, Some("Mon"), Some(10)));
    // Ordner: Datei da und unverändert, also überspringen
    assert!(decide_skip(
        true,
        false,
        Some(&e),
        Some(10),
        None,
        Some("Mon"),
        Some(10)
    ));
    // Zip: lokale Datei irrelevant
    assert!(decide_skip(true, true, Some(&e), None, None, Some("Mon"), Some(10)));
    // geändert: laden
    assert!(!decide_skip(
        true,
        false,
        Some(&e),
        Some(10),
        None,
        Some("Tue"),
        Some(10)
    ));
    // kein Indexeintrag, gleiche Größe: überspringen (Altbestand)
    assert!(decide_skip(true, false, None, Some(5), None, None, Some(5)));
    assert!(!decide_skip(true, false, None, Some(5), None, None, Some(6)));
    assert!(!decide_skip(true, true, None, None, None, None, Some(5)));
}

#[test]
fn index_roundtrip_and_key() {
    let dir = std::env::temp_dir().join(format!("ms-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut idx = Index::new();
    idx.insert(
        index_key("5", &["W1".into(), "A.pdf".into()]),
        entry(3, Some("x"), None),
    );
    save_index(&dir, &idx);
    let loaded = load_index(&dir);
    assert_eq!(loaded, idx);
    assert!(loaded.contains_key("5|w1/a.pdf"));
    let _ = fs::remove_dir_all(&dir);
    assert!(load_index(&dir.join("gibt-es-nicht")).is_empty());
}

#[test]
fn part_path_appends_suffix() {
    assert_eq!(part_path(Path::new("/a/b/x.pdf")), PathBuf::from("/a/b/x.pdf.part"));
}

#[test]
fn zip_method_by_extension() {
    assert!(matches!(zip_method("a.pdf"), CompressionMethod::Stored));
    assert!(matches!(zip_method("a.txt"), CompressionMethod::Deflated));
}

// Integrationstests gegen den lokalen Testserver

fn ctx_for(addr: std::net::SocketAddr) -> Ctx {
    Ctx::new(Url::parse(&format!("http://{addr}/")).unwrap(), "MoodleSession=abc").unwrap()
}

fn shared() -> Shared {
    Shared::new(1, 1, 3)
}

fn task(url: String, filename: &str, tmp: PathBuf) -> Task {
    Task {
        item: DownloadItem {
            section: "S".into(),
            section_index: 0,
            folder: None,
            filename: filename.into(),
            url,
            size: None,
        },
        key: "1|s/x".into(),
        rel_str: format!("S/{filename}"),
        tmp,
        final_path: None,
        entry: None,
        existing: None,
    }
}

fn tmpdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ms-it-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

const OPTS: Opts = Opts {
    incremental: true,
    zip: false,
    max_bytes: None,
};

#[tokio::test]
async fn downloads_file_and_sends_cookie() {
    let addr = spawn(|r| {
        assert!(r.head.to_lowercase().contains("cookie: moodlesession=abc"));
        resp(
            200,
            &[("ETag", "\"v1\""), ("Content-Type", "application/pdf")],
            b"hello",
        )
    })
    .await;
    let dir = tmpdir("ok");
    let t = task(format!("http://{addr}/f.pdf"), "f.pdf", dir.join("f.pdf.part"));
    let sh = shared();
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &sh)
        .await
        .unwrap();
    match r {
        Done::Downloaded { size, etag, .. } => {
            assert_eq!(size, 5);
            assert_eq!(etag.as_deref(), Some("\"v1\""));
        }
        _ => panic!("erwartet Downloaded"),
    }
    assert_eq!(fs::read(dir.join("f.pdf.part")).unwrap(), b"hello");
    assert_eq!(sh.bytes.load(Ordering::Relaxed), 5);
    assert!(sh.active.lock().unwrap().is_empty());
}

#[tokio::test]
async fn login_page_instead_of_file_means_session_expired() {
    let addr = spawn(|_| {
        resp(
            200,
            &[("Content-Type", "text/html; charset=utf-8")],
            b"<html><form action=\"/login/index.php\"></form></html>",
        )
    })
    .await;
    let dir = tmpdir("html");
    let t = task(format!("http://{addr}/f.pdf"), "f.pdf", dir.join("f.part"));
    let sh = shared();
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &sh).await;
    assert_eq!(r.err().unwrap(), crate::session::SESSION_EXPIRED);
    assert!(sh.expired.load(Ordering::Relaxed));
    assert!(!dir.join("f.part").exists());

    // Folgeaufgaben werden nicht mehr versucht
    let t2 = task(format!("http://{addr}/g.pdf"), "g.pdf", dir.join("g.part"));
    let r2 = run_task(&ctx_for(addr), &t2, OPTS, &AtomicBool::new(false), &sh).await;
    assert_eq!(r2.err().unwrap(), crate::session::SESSION_EXPIRED);
}

#[tokio::test]
async fn other_html_page_is_a_plain_error() {
    let addr = spawn(|_| {
        resp(
            200,
            &[("Content-Type", "text/html")],
            b"<html><a href=\"/login/logout.php\">Logout</a>Seite</html>",
        )
    })
    .await;
    let dir = tmpdir("html2");
    let t = task(format!("http://{addr}/f.pdf"), "f.pdf", dir.join("f.part"));
    let sh = shared();
    let e = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &sh)
        .await
        .err()
        .unwrap();
    assert!(e.contains("Webseite"), "{e}");
    assert!(!sh.expired.load(Ordering::Relaxed));
}

#[tokio::test]
async fn rotated_session_cookie_is_used_for_following_requests() {
    let addr = spawn(|r| {
        if r.path == "/first" {
            resp(200, &[("Set-Cookie", "MoodleSession=rotated; Path=/")], b"a")
        } else {
            let head = r.head.to_lowercase();
            assert!(head.contains("moodlesession=rotated"), "Cookie nicht rotiert: {head}");
            resp(200, &[], b"b")
        }
    })
    .await;
    let ctx = ctx_for(addr);
    ctx.get(&Url::parse(&format!("http://{addr}/first")).unwrap())
        .send()
        .await
        .unwrap();
    let r = ctx
        .get(&Url::parse(&format!("http://{addr}/second")).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(ctx.cookie_header().unwrap().contains("rotated"));
}

#[test]
fn throttle_halves_parallelism_down_to_one() {
    let sh = Shared::new(10, 1, 4);
    assert_eq!(sh.capacity(), 4);
    assert!(sh.throttle());
    assert_eq!(sh.capacity(), 2);
    assert_eq!(sh.permits.available_permits(), 2);
    assert!(sh.throttle());
    assert_eq!(sh.capacity(), 1);
    assert!(!sh.throttle());
    assert_eq!(sh.permits.available_permits(), 1);
}

#[tokio::test]
async fn overload_response_throttles_concurrency() {
    let addr = spawn(|r| {
        if r.nth == 0 {
            resp(429, &[("Retry-After", "0")], b"")
        } else {
            resp(200, &[], b"ok")
        }
    })
    .await;
    let dir = tmpdir("throttle");
    let t = task(format!("http://{addr}/f.bin"), "f.bin", dir.join("f.part"));
    let sh = Shared::new(1, 1, 4);
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &sh)
        .await
        .unwrap();
    assert!(matches!(r, Done::Downloaded { .. }));
    assert_eq!(sh.capacity(), 2);
}

#[tokio::test]
async fn retries_server_errors_then_succeeds() {
    let addr = spawn(|r| {
        if r.nth == 0 {
            resp(503, &[("Retry-After", "0")], b"")
        } else {
            resp(200, &[], b"data")
        }
    })
    .await;
    let dir = tmpdir("retry");
    let t = task(format!("http://{addr}/f.bin"), "f.bin", dir.join("f.part"));
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared())
        .await
        .unwrap();
    assert!(matches!(r, Done::Downloaded { size: 4, .. }));
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let addr = spawn(|_| resp(500, &[], b"")).await;
    let dir = tmpdir("giveup");
    let t = task(format!("http://{addr}/f.bin"), "f.bin", dir.join("f.part"));
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared()).await;
    let e = r.err().unwrap();
    assert!(e.contains("HTTP 500") && e.contains("3 Versuchen"), "{e}");
}

#[tokio::test]
async fn does_not_retry_client_errors() {
    let addr = spawn(|_| resp(404, &[], b"")).await;
    let dir = tmpdir("404");
    let t = task(format!("http://{addr}/nope"), "nope.pdf", dir.join("f.part"));
    let e = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared())
        .await
        .err()
        .unwrap();
    assert_eq!(e, "HTTP 404");
}

#[tokio::test]
async fn truncated_transfer_is_retried_and_bytes_balanced() {
    let addr = spawn(|r| {
        if r.nth == 0 {
            truncated(10, b"12345")
        } else {
            resp(200, &[], b"1234567890")
        }
    })
    .await;
    let dir = tmpdir("trunc");
    let t = task(format!("http://{addr}/f.bin"), "f.bin", dir.join("f.part"));
    let sh = shared();
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &sh)
        .await
        .unwrap();
    assert!(matches!(r, Done::Downloaded { size: 10, .. }));
    assert_eq!(sh.bytes.load(Ordering::Relaxed), 10); // Fehlversuch wurde zurückgerechnet
}

#[tokio::test]
async fn skips_unchanged_by_etag() {
    let addr = spawn(|_| resp(200, &[("ETag", "\"v1\"")], b"hello")).await;
    let dir = tmpdir("skip");
    let mut t = task(format!("http://{addr}/f.pdf"), "f.pdf", dir.join("f.part"));
    t.entry = Some(IndexEntry {
        size: 5,
        etag: Some("\"v1\"".into()),
        modified: None,
        url: "u".into(),
    });
    t.existing = Some(5);
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared())
        .await
        .unwrap();
    assert!(matches!(r, Done::Skipped));
    assert!(!dir.join("f.part").exists());

    // geänderter ETag: neu laden
    t.entry = Some(IndexEntry {
        size: 5,
        etag: Some("\"v0\"".into()),
        modified: None,
        url: "u".into(),
    });
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared())
        .await
        .unwrap();
    assert!(matches!(r, Done::Downloaded { .. }));
}

#[tokio::test]
async fn skips_files_over_size_limit() {
    let addr = spawn(|_| resp(200, &[], &[0u8; 2048])).await;
    let dir = tmpdir("big");
    let t = task(format!("http://{addr}/big.bin"), "big.bin", dir.join("f.part"));
    let opts = Opts {
        max_bytes: Some(1024),
        ..OPTS
    };
    let r = run_task(&ctx_for(addr), &t, opts, &AtomicBool::new(false), &shared())
        .await
        .unwrap();
    assert!(matches!(r, Done::Skipped));
    assert!(!dir.join("f.part").exists());
}

#[tokio::test]
async fn cancel_flag_stops_before_request() {
    let addr = spawn(|_| panic!("darf nicht abgefragt werden")).await;
    let dir = tmpdir("cancel");
    let t = task(format!("http://{addr}/f.bin"), "f.bin", dir.join("f.part"));
    let r = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(true), &shared())
        .await
        .unwrap();
    assert!(matches!(r, Done::Cancelled));
}

#[tokio::test]
async fn blocks_foreign_hosts() {
    let addr = spawn(|_| resp(200, &[], b"x")).await;
    let dir = tmpdir("foreign");
    let t = task("http://localhost:1/evil.pdf".into(), "evil.pdf", dir.join("f.part"));
    let e = run_task(&ctx_for(addr), &t, OPTS, &AtomicBool::new(false), &shared())
        .await
        .err()
        .unwrap();
    assert!(e.contains("blockiert"), "{e}");
}

#[test]
fn finalize_dir_renames_and_zip_writes_safe_entries() {
    let dir = tmpdir("final");
    // Ordner
    let tmp = dir.join("a.pdf.part");
    fs::write(&tmp, b"x").unwrap();
    let fin = dir.join("a.pdf");
    let mut t = task("http://h/a.pdf".into(), "a.pdf", tmp);
    t.final_path = Some(fin.clone());
    let mut sink = Sink::Dir(dir.clone());
    finalize(&mut sink, &t).unwrap();
    assert_eq!(fs::read(&fin).unwrap(), b"x");

    // ZIP
    let zpath = dir.join("k.zip");
    let mut zsink = Sink::Zip(Box::new(ZipWriter::new(File::create(&zpath).unwrap())));
    let ztmp = dir.join("1.part");
    fs::write(&ztmp, b"zipdata").unwrap();
    let mut zt = task("http://h/b.txt".into(), "b.txt", ztmp.clone());
    zt.rel_str = "Woche 1/b.txt".into();
    finalize(&mut zsink, &zt).unwrap();
    if let Sink::Zip(z) = zsink {
        z.finish().unwrap();
    }
    let mut ar = zip::ZipArchive::new(File::open(&zpath).unwrap()).unwrap();
    assert_eq!(ar.len(), 1);
    let f = ar.by_index(0).unwrap();
    assert_eq!(f.name().unwrap(), "Woche 1/b.txt");
    assert!(!ztmp.exists());
}

fn job(zip: bool, incremental: bool, max_bytes: Option<u64>) -> crate::types::DownloadJob {
    crate::types::DownloadJob {
        dest_dir: String::new(),
        zip,
        dir_template: "{section}".into(),
        incremental,
        parallel: 2,
        max_bytes,
        allow_executables: false,
        save_links: true,
        courses: vec![],
    }
}

#[test]
fn preview_classification() {
    let mut t = task("http://h/a.pdf".into(), "a.pdf", PathBuf::from("x.part"));
    let meta = |etag: Option<&str>, len: Option<u64>| super::preview::Meta {
        etag: etag.map(Into::into),
        modified: None,
        len,
    };
    // neu
    assert_eq!(
        classify(&job(false, true, None), &t, &meta(None, Some(10))),
        (Class::New, Some(10))
    );
    // geändert: Datei existiert lokal, aber ohne Indexeintrag und andere Größe
    t.existing = Some(5);
    assert_eq!(
        classify(&job(false, true, None), &t, &meta(None, Some(10))).0,
        Class::Changed
    );
    // unverändert: Indexeintrag stimmt überein
    t.entry = Some(IndexEntry {
        size: 10,
        etag: Some("e1".into()),
        modified: None,
        url: "u".into(),
    });
    t.existing = Some(10);
    assert_eq!(
        classify(&job(false, true, None), &t, &meta(Some("e1"), Some(10))).0,
        Class::Unchanged
    );
    // inkrementell aus: alles wird neu geladen
    assert_eq!(
        classify(&job(false, false, None), &t, &meta(Some("e1"), Some(10))).0,
        Class::Changed
    );
    // zu groß
    assert_eq!(
        classify(&job(false, true, Some(5)), &t, &meta(Some("e1"), Some(10))).0,
        Class::TooLarge
    );
    // Größe nur aus Scan bekannt
    t.item.size = Some(7);
    assert_eq!(classify(&job(false, true, None), &t, &meta(None, None)).1, Some(7));
}
