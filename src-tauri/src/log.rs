//! Kleines Protokoll in einer Datei: Tokens werden maskiert, bei 1 MB wird rotiert.

use crate::util::{iso_utc, now_ts, redact};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MAX_BYTES: u64 = 1_000_000;
const FILE_NAME: &str = "moodle-scraper.log";

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Einmal beim Start aufrufen.
pub fn init(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let _ = LOG_DIR.set(dir.to_path_buf());
}

fn path() -> Option<PathBuf> {
    LOG_DIR.get().map(|d| d.join(FILE_NAME))
}

/// Hängt eine Zeile an (`level`: INFO, WARN oder ERROR). Tokens und Sesskeys werden maskiert.
pub fn log(level: &str, msg: &str) {
    let Some(p) = path() else { return };
    write_line(&p, level, msg);
}

fn write_line(p: &Path, level: &str, msg: &str) {
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if fs::metadata(p).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let _ = fs::rename(p, p.with_extension("log.1"));
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(p) {
        let line = redact(msg).replace('\n', " ⏎ ");
        let _ = writeln!(f, "{} {level:<5} {line}", iso_utc(now_ts()));
    }
}

pub fn tail(n: usize) -> String {
    let Some(p) = path() else { return String::new() };
    tail_of(&p, n)
}

fn tail_of(p: &Path, n: usize) -> String {
    let text = fs::read_to_string(p).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_redacted_lines_and_rotates() {
        let dir = std::env::temp_dir().join(format!("ms-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join(FILE_NAME);

        write_line(&p, "INFO", "login ok token=GEHEIM123 weiter");
        write_line(&p, "ERROR", "zeile1\nzeile2");
        let t = tail_of(&p, 10);
        assert!(t.contains("INFO") && t.contains("token=***") && !t.contains("GEHEIM123"));
        assert!(t.contains("zeile1 ⏎ zeile2"));
        assert_eq!(tail_of(&p, 1).lines().count(), 1);

        // Zu groß geworden: alte Datei als .log.1 aufheben
        fs::write(&p, vec![b'x'; (MAX_BYTES + 10) as usize]).unwrap();
        write_line(&p, "INFO", "neu");
        assert!(p.with_extension("log.1").exists());
        assert!(tail_of(&p, 5).contains("neu"));
        let _ = fs::remove_dir_all(&dir);
    }
}
