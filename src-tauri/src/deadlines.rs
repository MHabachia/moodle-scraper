//! Anstehende Abgaben und Termine aus dem Moodle-Kalender, dazu der Export als .ics.

use crate::session::{ajax, AppState, SESSION_EXPIRED};
use crate::types::Deadline;
use crate::util::{build_ics, now_ts, validate_export_path};
use std::fs;
use std::path::Path;
use tauri::State;

const ONE_DAY: i64 = 86_400;
/// Moodle erlaubt für diese Abfrage höchstens 50 Einträge.
const EVENT_LIMIT: u32 = 50;

fn event_args(now: i64) -> serde_json::Value {
    serde_json::json!({
        "timesortfrom": now - ONE_DAY,
        "limitnum": EVENT_LIMIT,
        "limittononsuspendedevents": true
    })
}

fn id_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// Termine, die mehr als einen Tag zurückliegen, fallen heraus.
pub fn parse_events(data: &serde_json::Value, now: i64) -> Vec<Deadline> {
    let mut out: Vec<Deadline> = data["events"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|e| {
            let due = e["timesort"].as_i64()?;
            if due < now - ONE_DAY {
                return None;
            }
            let url = e["action"]["url"]
                .as_str()
                .or_else(|| e["url"].as_str())
                .unwrap_or_default()
                .to_string();
            Some(Deadline {
                id: id_string(&e["id"]),
                name: e["name"].as_str().unwrap_or("").trim().to_string(),
                course: e["course"]["fullname"].as_str().unwrap_or("").trim().to_string(),
                due,
                url,
                kind: e["eventtype"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();
    out.sort_by_key(|d| d.due);
    out
}

#[tauri::command]
pub async fn list_deadlines(state: State<'_, AppState>) -> Result<Vec<Deadline>, String> {
    let ctx = state.ctx()?;
    let now = now_ts();
    let data = ajax(&ctx, "core_calendar_get_action_events_by_timesort", event_args(now))
        .await
        .map_err(|e| {
            if e == SESSION_EXPIRED {
                e
            } else {
                format!("Fristen nicht verfügbar: {e}")
            }
        })?;
    Ok(parse_events(&data, now))
}

#[tauri::command]
pub fn export_ics(path: String, deadlines: Vec<Deadline>) -> Result<(), String> {
    let p = Path::new(&path);
    validate_export_path(p, &["ics"])?;
    fs::write(p, build_ics(&deadlines, now_ts())).map_err(|e| format!("Datei nicht schreibbar: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_request_stays_within_moodle_limit() {
        let args = event_args(1_000_000);
        assert!(args["limitnum"].as_u64().unwrap() <= 50);
        assert_eq!(args["timesortfrom"], 1_000_000 - ONE_DAY);
    }

    #[test]
    fn parses_sorts_and_filters_events() {
        let data = serde_json::json!({"events": [
            {"id": 2, "name": " Später ", "timesort": 1_002_000, "course": {"fullname": "B"},
             "url": "https://m/e2", "eventtype": "due", "action": {"url": "https://m/a2"}},
            {"id": "1", "name": "Früh", "timesort": 1_001_500, "course": {"fullname": "A"}, "url": "https://m/e1"},
            {"id": 3, "name": "Uralt", "timesort": 10, "course": {"fullname": "C"}, "url": "u"}
        ]});
        let d = parse_events(&data, 10_000_000);
        assert!(d.is_empty(), "alles weit in der Vergangenheit");
        let d = parse_events(&data, 1_000_000);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].name, "Früh");
        assert_eq!(d[0].url, "https://m/e1");
        assert_eq!(d[1].name, "Später");
        assert_eq!(d[1].url, "https://m/a2", "Aktions-URL hat Vorrang");
        assert_eq!(d[1].kind, "due");
        assert!(parse_events(&serde_json::json!({}), 0).is_empty());
    }

    fn validate_export_path_ics(p: &Path) -> Result<(), String> {
        validate_export_path(p, &["ics"])
    }

    #[test]
    fn ics_path_validation() {
        let (ok, upper, bad_ext, forbidden) = if cfg!(windows) {
            (
                "C:\\Users\\x\\Downloads\\fristen.ics",
                "C:\\Users\\x\\Downloads\\FRISTEN.ICS",
                "C:\\Users\\x\\Downloads\\fristen.txt",
                "C:\\Windows\\fristen.ics",
            )
        } else {
            (
                "/Users/x/Downloads/fristen.ics",
                "/Users/x/Downloads/FRISTEN.ICS",
                "/Users/x/Downloads/fristen.txt",
                "/etc/fristen.ics",
            )
        };
        assert!(validate_export_path_ics(Path::new(ok)).is_ok());
        assert!(validate_export_path_ics(Path::new(upper)).is_ok());
        assert!(validate_export_path_ics(Path::new(bad_ext)).is_err());
        assert!(validate_export_path_ics(Path::new(forbidden)).is_err());
        assert!(validate_export_path_ics(Path::new("fristen.ics")).is_err(), "relativ");
    }

    #[test]
    fn export_writes_file() {
        // Der macOS-Temp-Ordner liegt unter /var und ist als Ziel gesperrt, deshalb /tmp.
        let base = if cfg!(unix) {
            std::path::PathBuf::from("/tmp")
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("ms-ics-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f.ics");
        export_ics(p.to_string_lossy().to_string(), vec![]).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains("BEGIN:VCALENDAR"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
