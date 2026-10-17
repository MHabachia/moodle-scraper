//! Scan und Download gegen das Fake-Moodle aus `testutil`.

use super::*;
use crate::scan::scan_course_inner;
use crate::testutil::{fake_moodle, FakeMoodle};
use crate::types::{CourseContent, DownloadItem, JobCourse, JobLink};
use std::sync::atomic::AtomicBool;
use tauri::Url;

fn ctx(m: &FakeMoodle) -> Ctx {
    let mut c = Ctx::new(Url::parse(&format!("http://{}/", m.addr)).unwrap(), "MoodleSession=abc").unwrap();
    c.sesskey = "k".into();
    c
}

fn tmp(name: &str) -> PathBuf {
    // Der macOS-Temp-Ordner liegt unter /var und ist als Ziel gesperrt, deshalb /tmp.
    let base = if cfg!(unix) {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    };
    let d = base.join(format!("ms-e2e-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

fn job_from(content: &CourseContent, dest: &Path, zip: bool, incremental: bool) -> DownloadJob {
    let mut items = Vec::new();
    let mut links = Vec::new();
    for s in &content.sections {
        for it in &s.items {
            items.push(DownloadItem {
                section: s.title.clone(),
                section_index: s.number,
                folder: it.folder.clone(),
                filename: it.filename.clone(),
                url: it.url.clone(),
                size: it.size,
            });
        }
        for l in &s.links {
            links.push(JobLink {
                section: s.title.clone(),
                name: l.name.clone(),
                url: l.url.clone(),
            });
        }
    }
    DownloadJob {
        dest_dir: dest.to_string_lossy().to_string(),
        zip,
        dir_template: "{section}/{folder}".into(),
        incremental,
        parallel: 3,
        max_bytes: None,
        allow_executables: false,
        save_links: true,
        courses: vec![JobCourse {
            id: "5".into(),
            name: "Analysis 1".into(),
            items,
            links,
        }],
    }
}

async fn run(ctx: &Ctx, job: &DownloadJob, dest: &Path) -> DownloadSummary {
    let shared = Arc::new(Shared::new(
        job.courses.iter().map(|c| c.items.len()).sum(),
        job.courses.len(),
        job.parallel,
    ));
    run_job(ctx, job, dest, &shared, &Arc::new(AtomicBool::new(false)))
        .await
        .unwrap()
}

#[tokio::test]
async fn scan_finds_files_folders_assignments_and_links() {
    let m = fake_moodle().await;
    let content = scan_course_inner(&ctx(&m), "5").await.unwrap();

    let titles: Vec<_> = content.sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, vec!["Allgemeines", "Woche 1: Grenzwerte"]);

    let names: Vec<_> = content
        .sections
        .iter()
        .flat_map(|s| s.items.iter().map(|i| (i.filename.as_str(), i.size)))
        .collect();
    assert!(names.contains(&("Syllabus.pdf", Some(12))));
    assert!(names.contains(&("Folien 1.pdf", None)) || names.iter().any(|(n, _)| n.starts_with("Folien")));
    assert!(names.iter().any(|(n, _)| *n == "Beispiele.zip"));
    assert!(names.iter().any(|(n, _)| *n == "Blatt1.pdf"));
    assert!(names.iter().any(|(n, _)| *n == "Zusatz.pdf"));

    // Verschachtelter Ordner bleibt erhalten
    let beispiele = content
        .sections
        .iter()
        .flat_map(|s| s.items.iter())
        .find(|i| i.filename == "Beispiele.zip")
        .unwrap();
    assert_eq!(beispiele.folder.as_deref(), Some("Folien/Zusatz"));

    // Aus der URL-Aktivität wird ein Link; die externe Seite wird nicht aufgerufen.
    let links: Vec<_> = content.sections.iter().flat_map(|s| s.links.iter()).collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].url, "https://youtu.be/xyz");
    assert!(!m.requests.lock().unwrap().iter().any(|(_, p)| p.contains("youtu")));
}

#[tokio::test]
async fn full_cycle_folder_mode_with_delta_sync() {
    let m = fake_moodle().await;
    let ctx = ctx(&m);
    let content = scan_course_inner(&ctx, "5").await.unwrap();
    let dest = tmp("cycle");
    let job = job_from(&content, &dest, false, true);

    // Lauf 1: alles neu
    let s1 = run(&ctx, &job, &dest).await;
    let r1 = &s1.results[0];
    assert!(r1.failed.is_empty(), "{:?}", r1.failed);
    assert_eq!(r1.downloaded, 5);
    assert_eq!(r1.skipped, 0);
    let course = dest.join("Analysis 1");
    assert_eq!(
        fs::read(course.join("Allgemeines/Syllabus.pdf")).unwrap(),
        b"SYLLABUS-PDF"
    );
    assert_eq!(
        fs::read(course.join("Woche 1_ Grenzwerte/Übungsblatt 1/Blatt1.pdf")).unwrap(),
        b"BLATT1"
    );
    assert_eq!(
        fs::read(course.join("Woche 1_ Grenzwerte/Folien/Zusatz/Beispiele.zip")).unwrap(),
        b"PK-ZIPDATA"
    );
    let links = fs::read_to_string(course.join("Links.md")).unwrap();
    assert!(links.contains("[Vorlesung (Video)](https://youtu.be/xyz)"));
    assert!(dest.join(".moodle-scraper-index.json").exists());
    assert!(!dest.join(".moodle-scraper-tmp").exists());

    // Lauf 2: nichts geändert, also wird alles übersprungen
    let s2 = run(&ctx, &job, &dest).await;
    let r2 = &s2.results[0];
    assert_eq!((r2.downloaded, r2.updated, r2.skipped), (0, 0, 5), "{:?}", r2.failed);

    // Lauf 3: das Syllabus wurde auf dem Server aktualisiert, nur diese Datei kommt neu
    *m.syllabus_etag.lock().unwrap() = "\"s2\"".into();
    let s3 = run(&ctx, &job, &dest).await;
    let r3 = &s3.results[0];
    assert_eq!((r3.downloaded, r3.updated, r3.skipped), (0, 1, 4));

    // Lokal gelöschte Datei wird nachgeladen
    fs::remove_file(course.join("Woche 1_ Grenzwerte/Übungsblatt 1/Blatt1.pdf")).unwrap();
    let s4 = run(&ctx, &job, &dest).await;
    assert_eq!(s4.results[0].updated, 1);
    assert!(course.join("Woche 1_ Grenzwerte/Übungsblatt 1/Blatt1.pdf").exists());

    // Vorschau ohne Delta: alles wäre "geändert"
    let all = preview::preview(&ctx, &job_from(&content, &dest, false, false), &dest).await;
    assert_eq!(all[0].new_files + all[0].changed_files, 5);
    let delta = preview::preview(&ctx, &job, &dest).await;
    assert_eq!(delta[0].unchanged, 5);
    assert_eq!(delta[0].links, 1);
    let _ = fs::remove_dir_all(&dest);
}

#[tokio::test]
async fn zip_mode_contains_files_and_links() {
    let m = fake_moodle().await;
    let ctx = ctx(&m);
    let content = scan_course_inner(&ctx, "5").await.unwrap();
    let dest = tmp("zip");
    let job = job_from(&content, &dest, true, true);
    let s = run(&ctx, &job, &dest).await;
    assert!(s.results[0].failed.is_empty(), "{:?}", s.results[0].failed);

    let zip_path = dest.join("Analysis 1.zip");
    let mut ar = zip::ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
    let names: Vec<String> = (0..ar.len())
        .map(|i| ar.by_index(i).unwrap().name().unwrap().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "Allgemeines/Syllabus.pdf"), "{names:?}");
    assert!(
        names.iter().any(|n| n.ends_with("Folien/Zusatz/Beispiele.zip")),
        "{names:?}"
    );
    assert!(names.iter().any(|n| n == "Links.md"));
    assert!(
        names.iter().all(|n| !n.contains("..") && !n.starts_with('/')),
        "{names:?}"
    );
    assert!(!dest.join(".moodle-scraper-tmp").exists());
    let _ = fs::remove_dir_all(&dest);
}

#[tokio::test]
async fn expired_session_reports_and_marks_all_remaining_for_retry() {
    let m = fake_moodle().await;
    let ctx = ctx(&m);
    let content = scan_course_inner(&ctx, "5").await.unwrap();
    let dest = tmp("expired");
    let mut job = job_from(&content, &dest, false, true);
    // ein zweiter Kurs, der nach dem Ablauf gar nicht mehr anfängt
    let second = JobCourse {
        id: "6".into(),
        name: "Kurs 2".into(),
        items: job.courses[0].items.clone(),
        links: vec![],
    };
    job.courses.push(second);
    job.parallel = 1;

    m.expired.store(true, std::sync::atomic::Ordering::SeqCst);
    let s = run(&ctx, &job, &dest).await;
    assert!(s.session_expired);
    assert_eq!(s.results.len(), 2, "auch der nicht begonnene Kurs wird gemeldet");
    assert!(s.results[1]
        .failed
        .iter()
        .all(|f| f.item.is_some() && f.error == crate::session::SESSION_EXPIRED));
    assert_eq!(s.results[1].failed.len(), job.courses[1].items.len());

    // Nach dem erneuten Login genügt es, die fehlgeschlagenen Dateien zu wiederholen
    m.expired.store(false, std::sync::atomic::Ordering::SeqCst);
    let retry_courses: Vec<JobCourse> = s
        .results
        .iter()
        .map(|r| JobCourse {
            id: r.course_id.clone(),
            name: r.name.clone(),
            items: r.failed.iter().filter_map(|f| f.item.clone()).collect(),
            links: vec![],
        })
        .filter(|c| !c.items.is_empty())
        .collect();
    let retry = DownloadJob {
        courses: retry_courses,
        ..job_from(&content, &dest, false, true)
    };
    let s2 = run(&ctx, &retry, &dest).await;
    assert!(!s2.session_expired);
    assert!(s2.results.iter().all(|r| r.failed.is_empty()), "{:?}", s2.results);
    let _ = fs::remove_dir_all(&dest);
}
