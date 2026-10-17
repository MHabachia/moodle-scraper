//! iCalendar-Export (RFC 5545) für Fristen.

use super::time::ics_utc;
use crate::types::Deadline;

/// Text-Escaping nach RFC 5545 (Backslash, Semikolon, Komma, Zeilenumbruch).
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Faltet eine Zeile auf höchstens 75 Oktette (Fortsetzungszeilen beginnen mit Leerzeichen).
pub fn fold_line(line: &str) -> String {
    let mut out = String::new();
    let mut cur = 0usize;
    for c in line.chars() {
        let w = c.len_utf8();
        if cur + w > 75 {
            out.push_str("\r\n ");
            cur = 1;
        }
        out.push(c);
        cur += w;
    }
    out
}

fn safe_http(u: &str) -> bool {
    u.starts_with("https://") || u.starts_with("http://")
}

/// Baut den Kalender (CRLF-Zeilenenden).
pub fn build_ics(deadlines: &[Deadline], now: i64) -> String {
    let mut lines: Vec<String> = vec![
        "BEGIN:VCALENDAR".into(),
        "VERSION:2.0".into(),
        "PRODID:-//Moodle Scraper//Fristen//DE".into(),
        "CALSCALE:GREGORIAN".into(),
    ];
    for d in deadlines {
        let mut desc = format!("Kurs: {}", d.course);
        if safe_http(&d.url) {
            desc.push_str(&format!("\n{}", d.url));
        }
        lines.push("BEGIN:VEVENT".into());
        lines.push(format!("UID:{}-{}@moodle-scraper", escape_text(&d.id), d.due));
        lines.push(format!("DTSTAMP:{}", ics_utc(now)));
        lines.push(format!("DTSTART:{}", ics_utc(d.due)));
        lines.push(format!("DTEND:{}", ics_utc(d.due + 900)));
        lines.push(format!("SUMMARY:{}", escape_text(&d.name)));
        lines.push(format!("DESCRIPTION:{}", escape_text(&desc)));
        if safe_http(&d.url) {
            lines.push(format!("URL:{}", d.url.replace(['\r', '\n'], "")));
        }
        lines.push("END:VEVENT".into());
    }
    lines.push("END:VCALENDAR".into());
    lines.iter().map(|l| fold_line(l)).collect::<Vec<_>>().join("\r\n") + "\r\n"
}
