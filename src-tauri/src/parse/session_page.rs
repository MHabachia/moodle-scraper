//! Login-Status, Sesskey, Benutzername und Kursliste aus Moodle-Seiten.

use crate::types::Course;
use crate::util::{collapse_ws, html_unescape};
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;
use tauri::Url;

static RE_COURSE_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]id=(\d+)").unwrap());

static RE_SESSKEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""sesskey"\s*:\s*"([A-Za-z0-9]+)""#).unwrap());

static RE_USERTEXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"class="usertext[^"]*"[^>]*>([^<]+)<"#).unwrap());

pub fn is_logged_in(html: &str) -> bool {
    html.contains("login/logout.php") || html.contains("data-title=\"logout")
}

pub fn parse_sesskey(html: &str) -> Option<String> {
    RE_SESSKEY.captures(html).map(|c| c[1].to_string())
}

pub fn parse_user(html: &str) -> String {
    RE_USERTEXT
        .captures(html)
        .map(|c| html_unescape(c[1].trim()))
        .unwrap_or_default()
}

/// Kursliste aus dem HTML, falls AJAX nichts liefert.
pub fn parse_courses_html(html: &str, base: &Url) -> Vec<Course> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*='course/view.php?id=']").unwrap();
    let mut out: Vec<Course> = Vec::new();
    for a in doc.select(&sel) {
        let href = a.value().attr("href").unwrap_or_default();
        let Some(caps) = RE_COURSE_ID.captures(href) else {
            continue;
        };
        let id = caps[1].to_string();
        let text = collapse_ws(&a.text().collect::<String>());
        if let Some(existing) = out.iter_mut().find(|c| c.id == id) {
            if !text.is_empty() && existing.name.starts_with("Kurs ") && existing.name == format!("Kurs {id}") {
                existing.name = text;
            }
            continue;
        }
        out.push(Course {
            url: base
                .join(href)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| href.to_string()),
            name: if text.is_empty() { format!("Kurs {id}") } else { text },
            short_name: String::new(),
            category: String::new(),
            progress: None,
            favourite: false,
            hidden: false,
            ended: false,
            id,
        });
    }
    out
}
