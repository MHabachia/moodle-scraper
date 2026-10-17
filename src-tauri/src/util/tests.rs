//! Tests der Hilfsfunktionen.

use super::ics::{escape_text as ics_escape, fold_line};
use super::net::is_local_host;
use super::time::ics_utc;
use super::*;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use tauri::Url;

#[test]
fn normalize_adds_scheme_and_strips_paths() {
    assert_eq!(
        normalize_url("moodle.uni.de").unwrap().as_str(),
        "https://moodle.uni.de/"
    );
    assert_eq!(
        normalize_url("https://moodle.uni.de/my/courses.php?x=1")
            .unwrap()
            .as_str(),
        "https://moodle.uni.de/"
    );
    assert_eq!(
        normalize_url("https://uni.de/moodle/course/view.php?id=5")
            .unwrap()
            .as_str(),
        "https://uni.de/moodle/"
    );
}

#[test]
fn normalize_rejects_bad_input() {
    assert!(normalize_url("  ").is_err());
    assert!(normalize_url("ftp://x.de").is_err());
}

#[test]
fn sanitize_blocks_traversal_and_reserved() {
    assert_eq!(sanitize_component(".."), "_");
    assert_eq!(sanitize_component("../../etc"), "_.._etc"); // keine Trenner mehr -> harmlos
    assert_eq!(sanitize_component("a/b\\c:d"), "a_b_c_d");
    assert_eq!(sanitize_component("CON"), "_CON");
    assert_eq!(sanitize_component("com1.txt"), "_com1.txt");
    assert_eq!(sanitize_component("  Folien  1. "), "Folien 1");
    assert!(sanitize_component(&"x".repeat(300)).chars().count() <= 100);
}

#[test]
fn sanitize_filename_keeps_extension() {
    assert_eq!(sanitize_filename("Übung 1: Lösung?.pdf"), "Übung 1_ Lösung_.pdf");
    assert_eq!(sanitize_filename("noext"), "noext");
    assert_eq!(sanitize_filename(".hidden"), "hidden");
}

#[test]
fn template_rendering() {
    assert_eq!(
        render_dir_template("{section}/{folder}", "Woche 1", 1, Some("Skripte/Teil A")),
        vec!["Woche 1", "Skripte", "Teil A"]
    );
    assert_eq!(
        render_dir_template("{section}/{folder}", "Woche 1", 1, None),
        vec!["Woche 1"]
    );
    assert_eq!(
        render_dir_template("{n} - {section}", "Intro", 3, None),
        vec!["03 - Intro"]
    );
    assert!(render_dir_template("", "x", 0, None).is_empty());
    // Path-Traversal im Muster oder Abschnittsnamen wird neutralisiert
    let r = render_dir_template("../{section}", "..", 0, None);
    assert!(r.iter().all(|c| c != ".." && !c.contains('/')));
}

#[test]
fn unique_rel_dedupes_case_insensitive() {
    let mut used = HashSet::new();
    let a = unique_rel(&mut used, vec!["W1".into(), "Folie.pdf".into()]);
    let b = unique_rel(&mut used, vec!["W1".into(), "folie.PDF".into()]);
    let c = unique_rel(&mut used, vec!["W2".into(), "Folie.pdf".into()]);
    assert_eq!(a, vec!["W1", "Folie.pdf"]);
    assert_eq!(b, vec!["W1", "folie (2).PDF"]);
    assert_eq!(c, vec!["W2", "Folie.pdf"]);
}

#[test]
fn content_disposition_variants() {
    assert_eq!(
        parse_content_disposition(r#"attachment; filename="a b.pdf"; filename*=UTF-8''%C3%9Cbung.pdf"#),
        Some("Übung.pdf".into())
    );
    assert_eq!(
        parse_content_disposition(r#"attachment; filename="x.pdf""#),
        Some("x.pdf".into())
    );
    assert_eq!(
        parse_content_disposition("attachment; filename=y.zip"),
        Some("y.zip".into())
    );
    assert_eq!(parse_content_disposition("inline"), None);
}

#[test]
fn filename_from_url_decodes() {
    let u =
        Url::parse("https://m.de/pluginfile.php/1/mod_resource/content/2/Skript%20%C3%9C.pdf?forcedownload=1").unwrap();
    assert_eq!(filename_from_url(&u), "Skript Ü.pdf");
}

#[test]
fn bytes_formatting() {
    assert_eq!(fmt_bytes(0), "0 B");
    assert_eq!(fmt_bytes(1536), "1.5 KB");
    assert_eq!(fmt_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
}

#[test]
fn lock_survives_poisoning() {
    let m = std::sync::Arc::new(Mutex::new(1));
    let m2 = m.clone();
    let _ = std::thread::spawn(move || {
        let _g = m2.lock().unwrap();
        panic!("absichtlich");
    })
    .join();
    assert_eq!(*lock(&m), 1);
}

#[test]
fn ext_detection() {
    assert_eq!(ext_of("a.PDF"), "pdf");
    assert_eq!(ext_of(".bashrc"), "");
    assert_eq!(ext_of("archiv.tar.gz"), "gz");
}

#[test]
fn https_only_except_localhost() {
    assert!(normalize_url("http://moodle.uni.de").is_err());
    assert!(normalize_url("http://localhost:8080/moodle").is_ok());
    assert!(normalize_url("https://moodle.uni.de").is_ok());
}

#[test]
fn private_hosts_are_detected() {
    for h in [
        "localhost",
        "127.0.0.1",
        "10.1.2.3",
        "192.168.0.5",
        "172.16.0.1",
        "169.254.169.254",
        "100.64.0.1",
        "[::1]",
        "fd00::1",
        "fe80::1",
        "printer.local",
        "db.internal",
    ] {
        assert!(is_private_host(h), "{h} sollte privat sein");
    }
    for h in ["moodle.uni.de", "8.8.8.8", "172.32.0.1", "2001:db8::1"] {
        assert!(!is_private_host(h), "{h} sollte öffentlich sein");
    }
    assert!(is_local_host("localhost") && is_local_host("127.0.0.1") && !is_local_host("10.0.0.1"));
}

#[test]
fn url_allowed_requires_same_origin() {
    let base = Url::parse("https://moodle.uni.de/").unwrap();
    let ok = Url::parse("https://moodle.uni.de/pluginfile.php/1/a.pdf").unwrap();
    assert!(url_allowed(&base, &ok));
    for bad in [
        "https://evil.example/pluginfile.php/1/a.pdf",
        "http://moodle.uni.de/pluginfile.php/1/a.pdf",
        "https://moodle.uni.de:8443/pluginfile.php/1/a.pdf",
        "https://moodle.uni.de.evil.example/x",
        "https://169.254.169.254/latest/meta-data",
    ] {
        assert!(!url_allowed(&base, &Url::parse(bad).unwrap()), "{bad}");
    }
}

#[test]
fn executables_detected() {
    for e in ["exe", "SH", ".bat", "jar", "docm", "app", "dmg"] {
        assert!(is_executable_ext(e), "{e}");
    }
    for e in ["pdf", "docx", "pptx", "zip", "py", "txt", ""] {
        assert!(!is_executable_ext(e), "{e}");
    }
}

#[test]
fn secrets_are_redacted() {
    assert_eq!(
        redact("GET https://m.de/tokenpluginfile.php/abc123DEF/5/mod_resource/x.pdf failed"),
        "GET https://m.de/tokenpluginfile.php/***/5/mod_resource/x.pdf failed"
    );
    assert_eq!(
        redact("https://m.de/x?token=SECRET&a=1"),
        "https://m.de/x?token=***&a=1"
    );
    assert_eq!(redact("sesskey=AbC wstoken=zzz"), "sesskey=*** wstoken=***");
    assert_eq!(
        redact_url("https://m.de/pluginfile.php/1/a.pdf?forcedownload=1&token=XYZ#f"),
        "https://m.de/pluginfile.php/1/a.pdf"
    );
    assert_eq!(
        redact_url("https://m.de/tokenpluginfile.php/SECRET/1/a.pdf"),
        "https://m.de/tokenpluginfile.php/***/1/a.pdf"
    );
}

#[cfg(unix)]
#[test]
fn forbidden_destinations() {
    for p in [
        "/",
        "/etc",
        "/System/Library",
        "/usr/local/bin",
        "/Applications",
        "/var/root",
        "/Library/Foo",
        "relative/path",
        "/Users/x/../../etc",
    ] {
        assert!(is_forbidden_dest(Path::new(p)), "{p} sollte verboten sein");
    }
    for p in [
        "/Users/mo/Downloads/Moodle",
        "/Volumes/USB/Kurse",
        "/private/tmp/x",
        "/home/u/moodle",
    ] {
        assert!(!is_forbidden_dest(Path::new(p)), "{p} sollte erlaubt sein");
    }
}

#[cfg(windows)]
#[test]
fn forbidden_destinations() {
    for p in [
        "C:\\Windows\\System32",
        "C:\\Program Files\\App",
        "D:\\",
        "relative\\path",
    ] {
        assert!(is_forbidden_dest(Path::new(p)), "{p} sollte verboten sein");
    }
    for p in ["C:\\Users\\mo\\Downloads\\Moodle", "D:\\Studium"] {
        assert!(!is_forbidden_dest(Path::new(p)), "{p} sollte erlaubt sein");
    }
}

#[test]
fn nfc_normalization_unifies_names() {
    let nfd = "Uebung A\u{308}.pdf"; // A + kombinierender Umlaut
    let nfc = "Uebung \u{c4}.pdf";
    assert_eq!(sanitize_filename(nfd), sanitize_filename(nfc));
    assert_eq!(sanitize_component("Mu\u{308}nchen"), "M\u{fc}nchen");
}

#[test]
fn fit_components_limits_total_length() {
    let comps = vec!["A".repeat(100), "B".repeat(100), format!("{}.pdf", "C".repeat(100))];
    let fitted = fit_components(80, comps.clone(), 240);
    let total = 80 + fitted.iter().map(|c| c.chars().count() + 1).sum::<usize>();
    assert!(total <= 240, "zu lang: {total}");
    assert!(fitted.last().unwrap().ends_with(".pdf"));
    assert_eq!(fitted.len(), 3);

    // Kurzer Pfad bleibt unverändert
    let short = vec!["Woche 1".to_string(), "a.pdf".to_string()];
    assert_eq!(fit_components(50, short.clone(), 240), short);
}

#[test]
fn fit_components_handles_extreme_base() {
    let comps = vec!["Ordner".to_string(), "x".repeat(200) + ".pdf"];
    let fitted = fit_components(230, comps, 240);
    assert!(fitted.last().unwrap().ends_with(".pdf"));
    assert!(!fitted.iter().any(|c| c.is_empty()));
}

#[test]
fn utc_formatting() {
    assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(iso_utc(1_700_000_000), "2023-11-14T22:13:20Z");
    assert_eq!(ics_utc(1_700_000_000), "20231114T221320Z");
    // Schaltjahr
    assert_eq!(iso_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    assert_eq!(iso_utc(-1), "1969-12-31T23:59:59Z");
}

#[test]
fn ics_escaping_and_folding() {
    assert_eq!(ics_escape("a;b,c\\d\ne"), "a\\;b\\,c\\\\d\\ne");
    let long = "x".repeat(200);
    let folded = fold_line(&long);
    assert!(folded.split("\r\n").all(|l| l.len() <= 75));
    assert_eq!(folded.replace("\r\n ", ""), long);
    // Mehrbyte-Zeichen werden nicht zerschnitten
    let umlauts = "ä".repeat(100);
    assert_eq!(fold_line(&umlauts).replace("\r\n ", ""), umlauts);
}

#[test]
fn ics_calendar_structure() {
    let d = crate::types::Deadline {
        id: "42".into(),
        name: "Abgabe: Blatt 1, Teil A".into(),
        course: "Analysis; 1".into(),
        due: 1_700_000_000,
        url: "https://m.de/mod/assign/view.php?id=7".into(),
        kind: "due".into(),
    };
    let ics = build_ics(std::slice::from_ref(&d), 1_700_000_100);
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(ics.ends_with("END:VCALENDAR\r\n"));
    assert!(ics.contains("DTSTART:20231114T221320Z"));
    assert!(ics.contains("DTEND:20231114T222820Z"));
    assert!(ics.contains("SUMMARY:Abgabe: Blatt 1\\, Teil A"));
    assert!(ics.contains("UID:42-1700000000@moodle-scraper"));
    assert!(ics.contains("URL:https://m.de/mod/assign/view.php?id=7"));
    // unsichere Links erscheinen nicht
    let bad = crate::types::Deadline {
        url: "javascript:alert(1)".into(),
        ..d
    };
    assert!(!build_ics(&[bad], 0).contains("javascript"));
}
