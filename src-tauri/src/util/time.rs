//! Minimale UTC-Zeitfunktionen ohne zusätzliche Abhängigkeit.

use std::time::{SystemTime, UNIX_EPOCH};

/// Aktuelle Unix-Zeit in Sekunden.
pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Datum aus Tagen seit 1970-01-01 (Algorithmus von Howard Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// (Jahr, Monat, Tag, Stunde, Minute, Sekunde) in UTC.
pub fn utc_parts(ts: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400) as u32;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// "2023-11-14T22:13:20Z"
pub fn iso_utc(ts: i64) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(ts);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// "20231114T221320Z" (iCalendar)
pub fn ics_utc(ts: i64) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(ts);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}
