//! Dateien auf Ordner-, Aufgaben- und Ressourcen-Seiten.

use percent_encoding::percent_decode_str;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tauri::Url;

pub fn is_course_file(href: &str) -> bool {
    if !href.contains("pluginfile.php") {
        return false;
    }
    // Theme-/Systemdateien und Profilbilder sind nie Kursmaterial
    if ["/theme_", "/core_", "/course/overviewfiles", "/block_", "/user/"]
        .iter()
        .any(|p| href.contains(p))
    {
        return false;
    }
    // In Labels sind Bilder meist Dekoration, verlinkte Dokumente dagegen Material
    if href.contains("/mod_label/") {
        let path = href.split(['?', '#']).next().unwrap_or(href).to_lowercase();
        return ![".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"]
            .iter()
            .any(|e| path.ends_with(e));
    }
    true
}

fn folder_dirs(url: &Url, marker: &str) -> Vec<String> {
    // …/{marker}/{itemid}/{dirs…}/{file}
    let segs: Vec<String> = url
        .path_segments()
        .map(|s| {
            s.map(|p| percent_decode_str(p).decode_utf8_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    match segs.iter().position(|s| s == marker) {
        Some(i) if segs.len() > i + 3 => segs[i + 2..segs.len() - 1].to_vec(),
        _ => vec![],
    }
}

/// Dateien eines Moodle-Ordners: (URL, Unterordner-Pfad)
pub fn parse_folder_page(html: &str, page: &Url) -> Vec<(String, Vec<String>)> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*='pluginfile.php']").unwrap();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let href = a.value().attr("href").unwrap_or_default();
        if !is_course_file(href) || !href.contains("mod_folder") {
            continue;
        }
        let Ok(url) = page.join(href) else { continue };
        if !seen.insert(url.path().to_string()) {
            continue;
        }
        let dirs = folder_dirs(&url, "content");
        out.push((url.to_string(), dirs));
    }
    out
}

/// Anhänge der Aufgabenbeschreibung (nicht die Abgaben der Studierenden).
pub fn parse_assign_page(html: &str, page: &Url) -> Vec<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*='pluginfile.php']").unwrap();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let href = a.value().attr("href").unwrap_or_default();
        if !is_course_file(href) || !href.contains("/mod_assign/") || href.contains("/assignsubmission_") {
            continue;
        }
        let Ok(url) = page.join(href) else { continue };
        if seen.insert(url.path().to_string()) {
            out.push(url.to_string());
        }
    }
    out
}

/// Datei, die eine Ressourcenseite einbettet ("Auf Seite einbetten").
pub fn find_embedded_file(html: &str, page: &Url) -> Option<Url> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(
        "a[href*='pluginfile.php'], object[data*='pluginfile.php'], embed[src*='pluginfile.php'], \
         iframe[src*='pluginfile.php'], source[src*='pluginfile.php']",
    )
    .unwrap();
    for el in doc.select(&sel) {
        let v = el.value();
        let raw = v.attr("href").or(v.attr("data")).or(v.attr("src"))?;
        if let Ok(u) = page.join(raw) {
            return Some(u);
        }
    }
    None
}

/// Linkziel auf der Seite einer URL-Aktivität (nur http/https).
pub fn parse_url_page(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(".urlworkaround a[href]").unwrap();
    doc.select(&sel)
        .filter_map(|a| a.value().attr("href"))
        .map(|h| h.trim().to_string())
        .find(|h| Url::parse(h).is_ok_and(|u| matches!(u.scheme(), "http" | "https")))
}
