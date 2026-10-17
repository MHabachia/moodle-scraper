//! Ausführbare Endungen und Maskierung von Geheimnissen.

use regex::Regex;
use std::sync::LazyLock;
use tauri::Url;

/// Endungen, die ausführbaren Code oder Makros enthalten können.
const EXECUTABLE_EXTS: &[&str] = &[
    "exe", "msi", "bat", "cmd", "com", "scr", "ps1", "vbs", "vbe", "wsf", "lnk", "reg", "dll", "sh", "command", "app",
    "dmg", "pkg", "jar", "apk", "deb", "rpm", "run", "bin", "appimage", "docm", "xlsm", "pptm", "dotm", "xlam", "ppam",
];

pub fn is_executable_ext(ext: &str) -> bool {
    EXECUTABLE_EXTS.contains(&ext.trim_start_matches('.').to_lowercase().as_str())
}

static RE_SECRET_QUERY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(token|wstoken|sesskey|key|auth|password)=[^&\s)\x22']+").unwrap());

static RE_TOKEN_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(tokenpluginfile\.php/)[^/\s?]+").unwrap());

/// Maskiert Tokens/Sesskeys in beliebigem Text (Fehlermeldungen, Logs).
pub fn redact(text: &str) -> String {
    let t = RE_TOKEN_PATH.replace_all(text, "${1}***");
    RE_SECRET_QUERY.replace_all(&t, "${1}=***").to_string()
}

/// URL für die Speicherung im Index: ohne Query/Fragment, Tokens maskiert.
pub fn redact_url(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut u) => {
            u.set_query(None);
            u.set_fragment(None);
            redact(u.as_str())
        }
        Err(_) => redact(url),
    }
}
