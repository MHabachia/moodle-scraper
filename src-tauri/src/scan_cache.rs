//! Cache für Kursstrukturen auf der Platte, damit der zweite Start schneller ist.
//! Es werden keine Cookies gespeichert und nie Inhalte mit Token-URLs.

use crate::types::CourseContent;
use crate::util::sanitize_component;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const TTL_SECS: i64 = 6 * 3600;

#[derive(Serialize, Deserialize)]
struct Entry {
    saved_at: i64,
    content: CourseContent,
}

fn file(dir: &Path, host: &str, course_id: &str) -> PathBuf {
    dir.join("scan").join(format!(
        "{}-{}.json",
        sanitize_component(host),
        sanitize_component(course_id)
    ))
}

/// Token-URLs dürfen nicht auf die Platte.
pub fn cacheable(content: &CourseContent) -> bool {
    !content
        .sections
        .iter()
        .flat_map(|s| s.items.iter())
        .any(|i| i.url.to_lowercase().contains("token"))
}

pub fn load(dir: &Path, host: &str, course_id: &str, now: i64) -> Option<CourseContent> {
    let text = fs::read_to_string(file(dir, host, course_id)).ok()?;
    let entry: Entry = serde_json::from_str(&text).ok()?;
    (now - entry.saved_at < TTL_SECS && now >= entry.saved_at).then_some(entry.content)
}

pub fn store(dir: &Path, host: &str, course_id: &str, content: &CourseContent, now: i64) {
    if !cacheable(content) {
        return;
    }
    let p = file(dir, host, course_id);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let entry = Entry {
        saved_at: now,
        content: content.clone(),
    };
    if let Ok(s) = serde_json::to_string(&entry) {
        let _ = fs::write(p, s);
    }
}

pub fn clear(dir: &Path) {
    let _ = fs::remove_dir_all(dir.join("scan"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Item, Section};

    fn content(url: &str) -> CourseContent {
        CourseContent {
            course_id: "5".into(),
            sections: vec![Section {
                title: "W1".into(),
                number: 1,
                items: vec![Item {
                    id: String::new(),
                    name: "a".into(),
                    filename: "a.pdf".into(),
                    ext: "pdf".into(),
                    url: url.into(),
                    size: Some(3),
                    kind: "resource".into(),
                    folder: None,
                }],
                links: vec![],
            }],
        }
    }

    #[test]
    fn roundtrip_ttl_and_clear() {
        let dir = std::env::temp_dir().join(format!("ms-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let c = content("https://m.de/pluginfile.php/1/a.pdf");
        store(&dir, "m.de", "5", &c, 1000);
        let loaded = load(&dir, "m.de", "5", 1000 + 60).unwrap();
        assert_eq!(loaded.sections[0].items[0].filename, "a.pdf");
        assert!(load(&dir, "m.de", "5", 1000 + TTL_SECS + 1).is_none(), "abgelaufen");
        assert!(load(&dir, "andere.de", "5", 1000).is_none());
        clear(&dir);
        assert!(load(&dir, "m.de", "5", 1000).is_none());
    }

    #[test]
    fn token_urls_are_never_cached() {
        let dir = std::env::temp_dir().join(format!("ms-cache2-{}", std::process::id()));
        let c = content("https://m.de/tokenpluginfile.php/SECRET/1/a.pdf");
        assert!(!cacheable(&c));
        store(&dir, "m.de", "6", &c, 1000);
        assert!(load(&dir, "m.de", "6", 1000).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
