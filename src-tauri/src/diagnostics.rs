//! Schreibt Version, System und das (schon maskierte) Protokoll in eine Textdatei.

use crate::log;
use crate::util::{iso_utc, now_ts, validate_export_path};
use std::fs;
use std::path::Path;

pub fn build_report(log_tail: &str) -> String {
    format!(
        "Moodle Scraper {}\nErstellt: {}\nSystem: {} {}\n\n--- Protokoll (letzte Zeilen) ---\n{}\n",
        env!("CARGO_PKG_VERSION"),
        iso_utc(now_ts()),
        std::env::consts::OS,
        std::env::consts::ARCH,
        if log_tail.is_empty() { "(leer)" } else { log_tail }
    )
}

#[tauri::command]
pub fn export_diagnostics(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    validate_export_path(p, &["txt", "log"])?;
    fs::write(p, build_report(&log::tail(300))).map_err(|e| format!("Datei nicht schreibbar: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_contains_version_and_log() {
        let r = build_report("zeile 1\nzeile 2");
        assert!(r.contains(env!("CARGO_PKG_VERSION")));
        assert!(r.contains("zeile 2"));
        assert!(build_report("").contains("(leer)"));
    }

    #[test]
    fn rejects_bad_paths() {
        assert!(export_diagnostics("/etc/diag.txt".into()).is_err());
        assert!(export_diagnostics("/tmp/diag.exe".into()).is_err());
    }
}
