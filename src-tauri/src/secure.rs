//! Optional: die Session im Passwortspeicher des Systems ablegen (macOS Keychain,
//! Windows Anmeldeinformationsverwaltung). Nur wenn der Nutzer es einschaltet.

use crate::util::now_ts;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "moodle-scraper";
/// Gespeicherte Sessions älter als 30 Tage werden ignoriert.
const MAX_AGE_SECS: i64 = 30 * 86_400;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Saved {
    pub cookie: String,
    pub sesskey: String,
    pub saved_at: i64,
}

pub fn encode(s: &Saved) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// Liest einen Eintrag und verwirft zu alte oder kaputte.
pub fn decode(text: &str, now: i64) -> Option<Saved> {
    let s: Saved = serde_json::from_str(text).ok()?;
    (!s.cookie.is_empty() && s.saved_at <= now && now - s.saved_at <= MAX_AGE_SECS).then_some(s)
}

fn entry(host: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, host).map_err(|e| e.to_string())
}

pub fn save(host: &str, cookie: &str, sesskey: &str) -> Result<(), String> {
    let s = Saved {
        cookie: cookie.to_string(),
        sesskey: sesskey.to_string(),
        saved_at: now_ts(),
    };
    entry(host)?.set_password(&encode(&s)).map_err(|e| e.to_string())
}

pub fn load(host: &str) -> Option<Saved> {
    let text = entry(host).ok()?.get_password().ok()?;
    decode(&text, now_ts())
}

pub fn delete(host: &str) {
    if let Ok(e) = entry(host) {
        let _ = e.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_expiry() {
        let s = Saved {
            cookie: "MoodleSession=abc".into(),
            sesskey: "k".into(),
            saved_at: 1000,
        };
        let text = encode(&s);
        assert_eq!(decode(&text, 1000 + 60), Some(s));
        assert!(decode(&text, 1000 + MAX_AGE_SECS + 1).is_none(), "zu alt");
        assert!(decode(&text, 500).is_none(), "aus der Zukunft");
        assert!(decode("kaputt", 1000).is_none());
        assert!(
            decode(r#"{"cookie":"","sesskey":"","saved_at":1000}"#, 1001).is_none(),
            "leer"
        );
    }
}
