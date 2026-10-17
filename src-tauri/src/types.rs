//! Typen, die zwischen Backend und Frontend ausgetauscht werden. Daraus erzeugt ts-rs
//! die TypeScript-Typen in `src/generated`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionInfo {
    pub base_url: String,
    pub host: String,
    pub user: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Course {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub url: String,
    pub category: String,
    pub progress: Option<f64>,
    pub favourite: bool,
    pub hidden: bool,
    pub ended: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub ext: String,
    pub url: String,
    #[ts(type = "number | null")]
    pub size: Option<u64>,
    pub kind: String,
    pub folder: Option<String>,
}

/// Externer Link aus einem Kurs (z. B. Aktivität "URL"), wird als `Links.md` gespeichert.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkItem {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Section {
    pub title: String,
    pub number: usize,
    pub items: Vec<Item>,
    #[serde(default)]
    pub links: Vec<LinkItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CourseContent {
    pub course_id: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadItem {
    pub section: String,
    #[serde(default)]
    pub section_index: usize,
    pub folder: Option<String>,
    pub filename: String,
    pub url: String,
    #[ts(type = "number | null")]
    pub size: Option<u64>,
}

/// Externer Link, der in `Links.md` des Kurses geschrieben wird.
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JobLink {
    pub section: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JobCourse {
    pub id: String,
    pub name: String,
    pub items: Vec<DownloadItem>,
    #[serde(default)]
    pub links: Vec<JobLink>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadJob {
    pub dest_dir: String,
    pub zip: bool,
    /// Unterordner-Muster innerhalb des Kursordners, z. B. "{section}/{folder}".
    pub dir_template: String,
    /// Nur neue/geänderte Dateien laden.
    pub incremental: bool,
    pub parallel: usize,
    /// Dateien über dieser Größe überspringen (None = unbegrenzt).
    #[ts(type = "number | null")]
    pub max_bytes: Option<u64>,
    /// Ausführbare Dateien/Makro-Dokumente nur auf ausdrücklichen Wunsch laden.
    #[serde(default)]
    pub allow_executables: bool,
    /// Externe Links je Kurs als `Links.md` speichern.
    #[serde(default)]
    pub save_links: bool,
    pub courses: Vec<JobCourse>,
}

#[derive(Debug, Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Failure {
    pub name: String,
    pub error: String,
    pub item: Option<DownloadItem>,
}

#[derive(Debug, Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CourseResult {
    pub course_id: String,
    pub name: String,
    pub path: String,
    pub downloaded: usize,
    pub updated: usize,
    pub skipped: usize,
    #[ts(type = "number")]
    pub bytes: u64,
    pub failed: Vec<Failure>,
}

#[derive(Debug, Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadSummary {
    pub results: Vec<CourseResult>,
    pub cancelled: bool,
    /// Die Moodle-Session ist während des Laufs abgelaufen (erneut anmelden und fortsetzen).
    pub session_expired: bool,
}

#[derive(Debug, Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Progress {
    pub course_index: usize,
    pub course_count: usize,
    pub course_name: String,
    pub files_done: usize,
    pub files_total: usize,
    #[ts(type = "number")]
    pub bytes_done: u64,
    pub active: Vec<String>,
}

/// Vorschau eines Kurses vor dem Download (was wäre neu/geändert?).
#[derive(Debug, Serialize, Clone, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoursePreview {
    pub course_id: String,
    pub name: String,
    pub new_files: usize,
    pub changed_files: usize,
    pub unchanged: usize,
    /// Wegen Dateityp (Programme) übersprungen.
    pub blocked: usize,
    /// Über dem Größenlimit.
    pub too_large: usize,
    /// Bekannte Gesamtgröße der zu ladenden Dateien.
    #[ts(type = "number")]
    pub bytes: u64,
    /// Zu ladende Dateien ohne bekannte Größe.
    pub unknown_size: usize,
    pub links: usize,
}

/// Anstehende Frist/Termin aus dem Moodle-Kalender.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Deadline {
    pub id: String,
    pub name: String,
    pub course: String,
    /// Unix-Zeit (Sekunden)
    #[ts(type = "number")]
    pub due: i64,
    pub url: String,
    pub kind: String,
}
