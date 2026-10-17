//! URLs, Hosts und Dateinamen aus HTTP-Antworten.

use percent_encoding::percent_decode_str;
use regex::Regex;
use std::net::IpAddr;
use std::sync::LazyLock;
use tauri::Url;

static RE_CD_STAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)filename\*\s*=\s*[^']*'[^']*'([^;]+)"#).unwrap());

static RE_CD_QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)filename\s*=\s*"([^"]+)""#).unwrap());

static RE_CD_PLAIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)filename\s*=\s*([^;"]+)"#).unwrap());

/// Macht aus der Eingabe die Basis-URL (mit abschließendem `/`).
pub fn normalize_url(input: &str) -> Result<Url, String> {
    let mut s = input.trim().to_string();
    if s.is_empty() {
        return Err("Bitte eine Moodle-URL eingeben.".into());
    }
    if !s.contains("://") {
        s = format!("https://{s}");
    }
    let mut u = Url::parse(&s).map_err(|_| "Ungültige URL.".to_string())?;
    if u.scheme() != "https" && u.scheme() != "http" {
        return Err("Nur http(s)-URLs werden unterstützt.".into());
    }
    let Some(host) = u.host_str() else {
        return Err("Ungültige URL.".into());
    };
    if u.scheme() == "http" && !is_local_host(host) {
        return Err("Nur verschlüsselte Verbindungen (https://) werden unterstützt.".into());
    }
    let path = u.path().to_string();
    let markers = [
        "/login/",
        "/my/",
        "/course/",
        "/mod/",
        "/pluginfile.php",
        "/index.php",
        "/user/",
        "/lib/",
    ];
    let cut = markers.iter().filter_map(|m| path.find(m)).min();
    let new_path = match cut {
        Some(i) => path[..i].to_string(),
        None => path.trim_end_matches('/').to_string(),
    };
    u.set_path(&format!("{new_path}/"));
    u.set_query(None);
    u.set_fragment(None);
    Ok(u)
}

/// localhost und Loopback sind für lokale Tests erlaubt (sonst nur https).
pub fn is_local_host(host: &str) -> bool {
    let h = host.trim_matches(['[', ']']).to_lowercase();
    if h == "localhost" || h.ends_with(".localhost") {
        return true;
    }
    h.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// Interne Adressen, auf die wir nicht weiterleiten.
pub fn is_private_host(host: &str) -> bool {
    let h = host.trim_matches(['[', ']']).to_lowercase();
    if h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local") || h.ends_with(".internal") {
        return true;
    }
    match h.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            let o = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || (o[0] == 100 && (64..128).contains(&o[1]))
        }
        Ok(IpAddr::V6(ip)) => {
            let seg = ip.segments();
            ip.is_loopback()
                || ip.is_unspecified()
                || (seg[0] & 0xfe00) == 0xfc00 // fc00::/7
                || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10
        }
        Err(_) => false,
    }
}

/// Nur Adressen mit demselben Schema, Host und Port wie Moodle dürfen abgerufen werden.
pub fn url_allowed(base: &Url, url: &Url) -> bool {
    url.scheme() == base.scheme()
        && url.host_str() == base.host_str()
        && url.port_or_known_default() == base.port_or_known_default()
}

pub fn parse_content_disposition(cd: &str) -> Option<String> {
    if let Some(c) = RE_CD_STAR.captures(cd) {
        return Some(percent_decode_str(c[1].trim()).decode_utf8_lossy().to_string());
    }
    if let Some(c) = RE_CD_QUOTED.captures(cd) {
        return Some(c[1].to_string());
    }
    RE_CD_PLAIN.captures(cd).map(|c| c[1].trim().to_string())
}

pub fn filename_from_url(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut s| s.next_back().map(|l| l.to_string()))
        .map(|l| percent_decode_str(&l).decode_utf8_lossy().to_string())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "datei".into())
}
