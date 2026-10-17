//! Tests der Parser.

use super::course::RawSection;
use super::files::is_course_file;
use super::*;
use tauri::Url;

fn base() -> Url {
    Url::parse("https://moodle.uni.de/").unwrap()
}

const MOODLE3: &str = r#"
<html><body>
<a href="/login/logout.php?sesskey=abc">Logout</a>
<ul>
  <li id="section-0" class="section main"><h3 class="sectionname">Allgemeines</h3>
    <ul><li class="activity resource modtype_resource" id="module-11">
      <a class="aalink" href="https://moodle.uni.de/mod/resource/view.php?id=11">
        <span class="instancename">Syllabus<span class="accesshide"> Datei</span></span></a>
    </li></ul>
  </li>
  <li id="section-1" class="section main"><h3 class="sectionname">Woche 1</h3>
    <ul>
      <li class="activity folder modtype_folder" id="module-21">
        <a href="/mod/folder/view.php?id=21"><span class="instancename">Skripte<span class="accesshide"> Verzeichnis</span></span></a>
      </li>
      <li class="activity forum modtype_forum" id="module-22">
        <a href="/mod/forum/view.php?id=22"><span class="instancename">Forum</span></a>
      </li>
      <li class="activity assign modtype_assign" id="module-23">
        <a href="/mod/assign/view.php?id=23"><span class="instancename">Blatt 1</span></a>
      </li>
    </ul>
    <div class="summary"><a href="https://moodle.uni.de/pluginfile.php/5/course/section/3/extra.pdf">Extra</a>
    <img src="/pluginfile.php/5/theme_boost/x.png"><a href="/pluginfile.php/9/theme_boost/logo.png">x</a></div>
  </li>
</ul></body></html>"#;

#[test]
fn course_page_sections_and_items() {
    let secs = parse_course_page(MOODLE3, &base());
    assert_eq!(secs.len(), 2);
    assert_eq!(secs[0].title, "Allgemeines");
    assert_eq!(secs[0].items.len(), 1);
    assert_eq!(secs[0].items[0].name, "Syllabus");
    assert_eq!(secs[0].items[0].kind, "resource");
    assert_eq!(secs[0].items[0].id, "11");

    assert_eq!(secs[1].title, "Woche 1");
    let kinds: Vec<_> = secs[1].items.iter().map(|i| i.kind.as_str()).collect();
    assert_eq!(kinds, vec!["folder", "assign", "file"]); // Forum ignoriert, Theme-Dateien gefiltert
    assert_eq!(secs[1].items[0].url, "https://moodle.uni.de/mod/folder/view.php?id=21");
}

#[test]
fn course_page_without_sections_uses_single_section() {
    let html = r#"<a href="/pluginfile.php/1/mod_resource/content/1/a.pdf">A</a>"#;
    let secs = parse_course_page(html, &base());
    assert_eq!(secs.len(), 1);
    assert_eq!(secs[0].title, "Kurs");
    assert_eq!(secs[0].items.len(), 1);
}

#[test]
fn folder_page_with_subdirs() {
    let html = r#"
    <a href="/pluginfile.php/7/mod_folder/content/0/Teil%20A/s1.pdf?forcedownload=1">s1</a>
    <a href="/pluginfile.php/7/mod_folder/content/0/top.pdf">top</a>
    <a href="/pluginfile.php/7/mod_folder/content/0/top.pdf">dup</a>
    <a href="/pluginfile.php/7/mod_label/intro/0/x.png">label</a>"#;
    let files = parse_folder_page(html, &base());
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].1, vec!["Teil A"]);
    assert!(files[1].1.is_empty());
}

#[test]
fn assign_page_only_intro_attachments() {
    let html = r#"
    <a href="/pluginfile.php/3/mod_assign/introattachment/0/Aufgabe.pdf">Aufgabe</a>
    <a href="/pluginfile.php/3/assignsubmission_file/submission_files/9/abgabe.pdf">Abgabe</a>"#;
    let files = parse_assign_page(html, &base());
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("Aufgabe.pdf"));
}

#[test]
fn embedded_file_detection() {
    let html = r#"<div class="resourceworkaround"><object data="/pluginfile.php/1/mod_resource/content/1/x.pdf"></object></div>"#;
    let u = find_embedded_file(html, &base()).unwrap();
    assert!(u.path().ends_with("x.pdf"));
    assert!(find_embedded_file("<p>nichts</p>", &base()).is_none());
}

#[test]
fn course_state_parsing() {
    let state = serde_json::json!({
        "section": [
            {"id": 1, "number": 0, "title": "", "cmlist": ["10"]},
            {"id": 2, "number": 1, "title": "Woche &amp; 1", "cmlist": [11, 12, 13]}
        ],
        "cm": [
            {"id": "10", "module": "resource", "name": "Folien", "url": "https://moodle.uni.de/mod/resource/view.php?id=10", "uservisible": true},
            {"id": 11, "module": "folder", "name": "Ordner", "url": "/mod/folder/view.php?id=11"},
            {"id": 12, "module": "forum", "name": "Forum", "url": "/mod/forum/view.php?id=12"},
            {"id": 13, "module": "resource", "name": "Versteckt", "url": "/mod/resource/view.php?id=13", "uservisible": false}
        ]
    });
    // als JSON-String (so liefert Moodle es)
    let secs = parse_course_state(&serde_json::Value::String(state.to_string()), &base()).unwrap();
    assert_eq!(secs.len(), 2);
    assert_eq!(secs[0].title, "Allgemein");
    assert_eq!(secs[1].title, "Woche & 1");
    assert_eq!(secs[1].items.len(), 1);
    assert_eq!(secs[1].items[0].kind, "folder");
    assert_eq!(secs[1].items[0].url, "https://moodle.uni.de/mod/folder/view.php?id=11");
}

#[test]
fn course_state_without_supported_items_is_none() {
    let state = serde_json::json!({"section": [{"id":1,"number":0,"title":"x","cmlist":[]}], "cm": []});
    assert!(parse_course_state(&state, &base()).is_none());
    assert!(parse_course_state(&serde_json::json!("kaputt"), &base()).is_none());
}

#[test]
fn merge_adds_embedded_files_to_matching_section() {
    let ajax = vec![
        RawSection {
            title: "Allgemein".into(),
            number: 0,
            items: vec![],
        },
        RawSection {
            title: "Woche 1".into(),
            number: 1,
            items: vec![],
        },
    ];
    let html = vec![RawSection {
        title: "woche 1".into(),
        number: 1,
        items: vec![RawItem {
            id: String::new(),
            name: "x".into(),
            kind: "file".into(),
            url: "https://m/p.pdf".into(),
        }],
    }];
    let merged = merge_extra_files(ajax, &html);
    assert_eq!(merged[1].items.len(), 1);
    assert!(merged[0].items.is_empty());
}

#[test]
fn courses_html_dedupes_and_prefers_titles() {
    let html = r#"
    <a href="/course/view.php?id=5"></a>
    <a href="https://moodle.uni.de/course/view.php?id=5">Analysis 1</a>
    <a href="/course/view.php?id=7">Physik</a>"#;
    let c = parse_courses_html(html, &base());
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].name, "Analysis 1");
    assert_eq!(c[1].id, "7");
}

#[test]
fn session_parsing() {
    let html = r#"<script>M.cfg = {"sesskey":"AbC123","userId":5};</script>
    <span class="usertext mr-1">Max M&amp;ustermann</span><a href="/login/logout.php">x</a>"#;
    assert_eq!(parse_sesskey(html).as_deref(), Some("AbC123"));
    assert_eq!(parse_user(html), "Max M&ustermann");
    assert!(is_logged_in(html));
    assert!(!is_logged_in("<form action='login/index.php'>"));
}

#[test]
fn url_page_extracts_external_target_only() {
    let html =
        r#"<div class="urlworkaround">Klicken Sie auf <a href="https://youtu.be/abc" onclick="x">Link</a></div>"#;
    assert_eq!(parse_url_page(html).as_deref(), Some("https://youtu.be/abc"));
    let bad = r#"<div class="urlworkaround"><a href="javascript:alert(1)">x</a></div>"#;
    assert_eq!(parse_url_page(bad), None);
    assert_eq!(parse_url_page("<p>nichts</p>"), None);
}

#[test]
fn course_page_detects_url_activities() {
    let html = r#"<li id="section-1" class="section"><h3 class="sectionname">W1</h3>
      <ul><li class="activity url modtype_url" id="module-9">
        <a href="/mod/url/view.php?id=9"><span class="instancename">Video<span class="accesshide"> URL</span></span></a>
      </li></ul></li>"#;
    let secs = parse_course_page(html, &Url::parse("https://moodle.uni.de/").unwrap());
    assert_eq!(secs[0].items.len(), 1);
    assert_eq!(secs[0].items[0].kind, "url");
    assert_eq!(secs[0].items[0].name, "Video");
}

// Fixtures: Kursseiten verschiedener Moodle-Versionen und Themes

const FIXTURE_MOODLE3: &str = include_str!("../../tests/fixtures/moodle3_topics_boost.html");
const MOODLE4: &str = include_str!("../../tests/fixtures/moodle4_course_cards.html");
const CLASSIC: &str = include_str!("../../tests/fixtures/moodle_classic_flat.html");
const FOLDER: &str = include_str!("../../tests/fixtures/folder_page.html");

fn base_example() -> Url {
    Url::parse("https://moodle.example.edu/").unwrap()
}

fn summary(secs: &[RawSection]) -> Vec<(String, Vec<(String, String)>)> {
    secs.iter()
        .map(|s| {
            (
                s.title.clone(),
                s.items.iter().map(|i| (i.kind.clone(), i.name.clone())).collect(),
            )
        })
        .collect()
}

#[test]
fn fixture_moodle3_topics_boost() {
    assert!(is_logged_in(FIXTURE_MOODLE3));
    let secs = parse_course_page(FIXTURE_MOODLE3, &base_example());
    let got = summary(&secs);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].0, "Allgemeines");
    assert_eq!(got[0].1, vec![("resource".to_string(), "Syllabus".to_string())]);
    assert_eq!(got[1].0, "Woche 1: Grenzwerte");
    assert_eq!(
        got[1].1,
        vec![
            ("folder".to_string(), "Folien".to_string()),
            ("assign".to_string(), "Übungsblatt 1".to_string()),
            ("url".to_string(), "Vorlesung (Video)".to_string()),
            ("file".to_string(), "Zusatzmaterial".to_string()),
        ]
    );
}

#[test]
fn fixture_moodle4_course_cards() {
    let secs = parse_course_page(MOODLE4, &base_example());
    let got = summary(&secs);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].0, "Allgemein");
    assert_eq!(got[0].1, vec![("resource".to_string(), "Skript Teil 1".to_string())]);
    assert_eq!(got[1].0, "1 Oktober - 7 Oktober");
    assert_eq!(
        got[1].1,
        vec![
            ("folder".to_string(), "Aufgaben".to_string()),
            ("file".to_string(), "Hinweis.docx".to_string()), // Dokument aus Label bleibt erhalten
        ]
    );
}

#[test]
fn fixture_classic_flat_page() {
    let secs = parse_course_page(CLASSIC, &base_example());
    assert_eq!(secs.len(), 1);
    assert_eq!(secs[0].title, "Kurs");
    let names: Vec<_> = secs[0].items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["Kapitel 1", "Kapitel 2"], "Profilbild wird ignoriert");
}

#[test]
fn fixture_folder_page() {
    let files = parse_folder_page(FOLDER, &base_example());
    assert_eq!(files.len(), 2);
    assert!(files[0].0.contains("Folien%201.pdf"));
    assert!(files[0].1.is_empty());
    assert!(files[1].0.contains("Beispiele.zip"));
    assert_eq!(files[1].1, vec!["Zusatz"]);
}

#[test]
fn label_images_are_decoration_but_documents_are_material() {
    assert!(!is_course_file(
        "https://m/pluginfile.php/1/mod_label/intro/Bild.PNG?x=1"
    ));
    assert!(is_course_file(
        "https://m/pluginfile.php/1/mod_label/intro/Hinweis.docx"
    ));
    assert!(!is_course_file("https://m/pluginfile.php/1/user/icon/f1"));
    assert!(!is_course_file("https://m/other/file.pdf"));
}
