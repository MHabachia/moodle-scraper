//! Pfad-Bereinigung, Namensmuster und Zielordner-Prüfung.

use super::text::collapse_ws;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// Macht einen Namen als Pfadbestandteil tauglich (Windows und macOS).
pub fn sanitize_component(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    // NFC: macOS liefert Umlaute oft zerlegt (NFD), so stimmen Namen und Index überein
    let t = collapse_ws(&cleaned).nfc().collect::<String>();
    let t = t.trim_matches(['.', ' ']).to_string();
    let t: String = t.chars().take(100).collect();
    let t = t.trim_end_matches(['.', ' ']).to_string();
    if t.is_empty() {
        return "_".into();
    }
    let stem = t.split('.').next().unwrap_or("").to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.chars().last().is_some_and(|c| c.is_ascii_digit()));
    if reserved {
        format!("_{t}")
    } else {
        t
    }
}

pub fn sanitize_filename(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.len() <= 10 => {
            let s = sanitize_component(stem);
            let e: String = ext.chars().filter(|c| c.is_alphanumeric()).collect();
            if e.is_empty() {
                s
            } else {
                format!("{s}.{e}")
            }
        }
        _ => sanitize_component(name),
    }
}

/// Löst ein Unterordner-Muster auf. Tokens: `{section}`, `{n}` (Abschnittsnr.), `{folder}`.
/// Leere Segmente entfallen. Der Dateiname wird vom Aufrufer angehängt.
pub fn render_dir_template(template: &str, section: &str, number: usize, folder: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for seg in template.split('/') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        if seg == "{folder}" {
            if let Some(f) = folder {
                for p in f.split('/').filter(|p| !p.trim().is_empty()) {
                    out.push(sanitize_component(p));
                }
            }
            continue;
        }
        let rendered = seg
            .replace("{section}", section)
            .replace("{n}", &format!("{number:02}"))
            .replace("{folder}", folder.unwrap_or(""));
        let r = collapse_ws(&rendered);
        if r.chars().all(|c| !c.is_alphanumeric()) {
            continue;
        }
        out.push(sanitize_component(&r));
    }
    out
}

/// Macht den relativen Pfad (letztes Element = Dateiname) innerhalb eines Laufs eindeutig.
pub fn unique_rel(used: &mut HashSet<String>, mut comps: Vec<String>) -> Vec<String> {
    let file = comps.pop().unwrap_or_else(|| "datei".into());
    let (stem, ext) = match file.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (file.clone(), String::new()),
    };
    let dir = comps.join("/").to_lowercase();
    let mut n = 1;
    let mut candidate = file;
    while !used.insert(format!("{dir}/{}", candidate.to_lowercase())) {
        n += 1;
        candidate = format!("{stem} ({n}){ext}");
    }
    comps.push(candidate);
    comps
}

pub fn unique_path(p: PathBuf) -> PathBuf {
    if !p.exists() {
        return p;
    }
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 2;
    loop {
        let c = p.with_file_name(format!("{stem} ({n}){ext}"));
        if !c.exists() {
            return c;
        }
        n += 1;
    }
}

/// Zielordner, in die nie geschrieben werden soll (System-/Programmordner, Wurzel).
pub fn is_forbidden_dest(p: &Path) -> bool {
    if !p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return true;
    }
    let mut s = p.to_string_lossy().replace('\\', "/").to_lowercase();
    // Laufwerksbuchstabe entfernen: "c:/windows" -> "/windows"
    if s.len() >= 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_alphabetic() {
        s = s[2..].to_string();
    }
    let s = s.trim_end_matches('/');
    if s.is_empty() {
        return true; // Wurzel
    }
    const FORBIDDEN: &[&str] = &[
        "/system",
        "/library",
        "/usr",
        "/bin",
        "/sbin",
        "/etc",
        "/var",
        "/private/etc",
        "/private/var/db",
        "/applications",
        "/dev",
        "/proc",
        "/sys",
        "/boot",
        "/windows",
        "/program files",
        "/program files (x86)",
        "/programdata",
    ];
    FORBIDDEN.iter().any(|f| s == *f || s.starts_with(&format!("{f}/")))
}

/// Kürzt Pfadbestandteile so, dass `base_len + Pfad` höchstens `max_total` Zeichen hat
/// (Windows: 260). Zuerst werden die längsten Ordnernamen gekürzt, zuletzt der Dateiname
/// (Endung bleibt erhalten). Das letzte Element ist der Dateiname.
pub fn fit_components(base_len: usize, mut comps: Vec<String>, max_total: usize) -> Vec<String> {
    const MIN_DIR: usize = 12;
    const MIN_STEM: usize = 12;
    let total = |c: &[String]| base_len + c.iter().map(|x| x.chars().count() + 1).sum::<usize>();
    let trim = |s: &str, n: usize| -> String {
        let t: String = s.chars().take(n).collect();
        let t = t.trim_end_matches(['.', ' ']).to_string();
        if t.is_empty() {
            "_".to_string()
        } else {
            t
        }
    };

    while total(&comps) > max_total && comps.len() > 1 {
        let dirs = comps.len() - 1;
        let Some((i, len)) = comps[..dirs]
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.chars().count()))
            .filter(|(_, l)| *l > MIN_DIR)
            .max_by_key(|(_, l)| *l)
        else {
            break;
        };
        let excess = total(&comps) - max_total;
        comps[i] = trim(&comps[i], (len - excess.min(len - MIN_DIR)).max(MIN_DIR));
    }

    if total(&comps) > max_total {
        if let Some(file) = comps.last().cloned() {
            let (stem, ext) = match file.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
                _ => (file.clone(), String::new()),
            };
            let stem_len = stem.chars().count();
            let excess = total(&comps) - max_total;
            let keep = stem_len.saturating_sub(excess).max(MIN_STEM.min(stem_len));
            let last = comps.len() - 1;
            comps[last] = format!("{}{ext}", trim(&stem, keep));
        }
    }
    comps
}

/// Prüft einen vom Nutzer gewählten Zieldateipfad für Exporte (Endung und Ordner).
pub fn validate_export_path(path: &Path, exts: &[&str]) -> Result<(), String> {
    let ext_ok = path
        .extension()
        .is_some_and(|e| exts.iter().any(|x| e.to_string_lossy().eq_ignore_ascii_case(x)));
    if !ext_ok {
        return Err(format!("Die Datei muss auf .{} enden.", exts.join(" oder .")));
    }
    match path.parent() {
        Some(parent) if !is_forbidden_dest(parent) => Ok(()),
        _ => Err("Dieser Speicherort ist nicht erlaubt.".into()),
    }
}
