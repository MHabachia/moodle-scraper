//! Wohin eine fertige Datei kommt: in einen Ordner oder in ein ZIP.

use super::fetch::Task;
use crate::util::{ext_of, is_executable_ext};
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub(super) fn zip_method(name: &str) -> CompressionMethod {
    match ext_of(name).as_str() {
        "zip" | "pdf" | "png" | "jpg" | "jpeg" | "gif" | "mp4" | "mp3" | "mov" | "mkv" | "webm" | "docx" | "pptx"
        | "xlsx" | "7z" | "rar" | "gz" | "m4a" | "odt" | "odp" => CompressionMethod::Stored,
        _ => CompressionMethod::Deflated,
    }
}

pub(super) fn part_path(final_path: &Path) -> PathBuf {
    let mut name: OsString = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    final_path.with_file_name(name)
}

/// Markiert ausführbare Dateien als "aus dem Internet" (Windows: Mark-of-the-Web,
/// macOS: Quarantäne), damit das System vor dem Öffnen warnt.
#[cfg(windows)]
pub(super) fn mark_downloaded(p: &Path) {
    let mut s = p.as_os_str().to_owned();
    s.push(":Zone.Identifier");
    let _ = fs::write(PathBuf::from(s), "[ZoneTransfer]\r\nZoneId=3\r\n");
}

#[cfg(target_os = "macos")]
pub(super) fn mark_downloaded(p: &Path) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let value = format!("0081;{ts:x};MoodleScraper;");
    let _ = xattr::set(p, "com.apple.quarantine", value.as_bytes());
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn mark_downloaded(_p: &Path) {}

/// Zielart ohne geöffnete Datei, für Planung und Vorschau.
pub(super) enum Layout {
    Dir(PathBuf),
    Zip,
}

pub(super) enum Sink {
    Dir(PathBuf),
    Zip(Box<ZipWriter<File>>),
}

impl Sink {
    pub(super) fn layout(&self) -> Layout {
        match self {
            Sink::Dir(p) => Layout::Dir(p.clone()),
            Sink::Zip(_) => Layout::Zip,
        }
    }
}

pub(super) fn finalize(sink: &mut Sink, task: &Task) -> Result<(), String> {
    match sink {
        Sink::Dir(_) => {
            let fin = task.final_path.as_ref().ok_or("Zielpfad fehlt")?;
            let _ = fs::remove_file(fin);
            fs::rename(&task.tmp, fin).map_err(|e| format!("Umbenennen fehlgeschlagen: {e}"))?;
            if is_executable_ext(&ext_of(&task.item.filename)) {
                mark_downloaded(fin);
            }
            Ok(())
        }
        Sink::Zip(z) => {
            let size = fs::metadata(&task.tmp).map(|m| m.len()).unwrap_or(0);
            let opts = SimpleFileOptions::default()
                .compression_method(zip_method(&task.item.filename))
                .large_file(size >= 0xFFFF_FFFE);
            z.start_file(task.rel_str.clone(), opts)
                .map_err(|e| format!("ZIP-Fehler: {e}"))?;
            let mut f = File::open(&task.tmp).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, z).map_err(|e| format!("ZIP-Schreibfehler: {e}"))?;
            let _ = fs::remove_file(&task.tmp);
            Ok(())
        }
    }
}
