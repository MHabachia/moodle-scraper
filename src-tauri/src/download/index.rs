//! Der Index merkt sich pro Datei Größe, ETag und Änderungsdatum der letzten Runde.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const INDEX_FILE: &str = ".moodle-scraper-index.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexEntry {
    pub size: u64,
    pub etag: Option<String>,
    pub modified: Option<String>,
    pub url: String,
}

pub type Index = HashMap<String, IndexEntry>;

pub fn load_index(dir: &Path) -> Index {
    fs::read_to_string(dir.join(INDEX_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_index(dir: &Path, index: &Index) {
    if let Ok(s) = serde_json::to_string_pretty(index) {
        let tmp = dir.join(format!("{INDEX_FILE}.tmp"));
        if fs::write(&tmp, s).is_ok() {
            let _ = fs::rename(&tmp, dir.join(INDEX_FILE));
        }
    }
}

pub fn index_key(course_id: &str, rel: &[String]) -> String {
    format!("{course_id}|{}", rel.join("/").to_lowercase())
}

/// Vergleicht die Server-Header mit dem Eintrag aus dem Index.
pub fn is_unchanged(entry: &IndexEntry, etag: Option<&str>, modified: Option<&str>, len: Option<u64>) -> bool {
    if let (Some(a), Some(b)) = (entry.etag.as_deref(), etag) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (entry.modified.as_deref(), modified) {
        return a == b && len.map_or(true, |l| l == entry.size);
    }
    len.is_some_and(|l| l == entry.size)
}

pub fn decide_skip(
    incremental: bool,
    zip: bool,
    entry: Option<&IndexEntry>,
    existing_size: Option<u64>,
    etag: Option<&str>,
    modified: Option<&str>,
    len: Option<u64>,
) -> bool {
    if !incremental {
        return false;
    }
    match entry {
        Some(e) => is_unchanged(e, etag, modified, len) && (zip || existing_size == Some(e.size)),
        // Kein Indexeintrag (von Hand kopiert oder aus einer älteren Version): bei gleicher Größe nicht neu laden
        None => !zip && len.is_some() && existing_size == len && existing_size != Some(0),
    }
}
