//! Baut die Datei `Links.md` mit den externen Links eines Kurses, gruppiert nach Abschnitt.

use crate::types::JobLink;
use tauri::Url;

/// Nur http(s), alles andere wird verworfen.
fn safe_url(u: &str) -> Option<String> {
    let url = Url::parse(u.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

/// Escaped Markdown-Sonderzeichen im Linktext.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '[' | ']' | '\\' | '`' | '*' | '_' | '<' | '>' => {
                out.push('\\');
                out.push(c);
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

/// Klammern und Leerzeichen in der URL kodieren, sonst bricht der Markdown-Link.
fn escape_url(u: &str) -> String {
    u.replace(' ', "%20").replace('(', "%28").replace(')', "%29")
}

/// `None`, wenn kein gültiger Link übrig bleibt.
pub fn links_markdown(course: &str, links: &[JobLink]) -> Option<String> {
    let mut sections: Vec<(&str, Vec<(String, String)>)> = Vec::new();
    for l in links {
        let Some(url) = safe_url(&l.url) else { continue };
        let name = if l.name.trim().is_empty() {
            url.clone()
        } else {
            escape_text(&l.name)
        };
        match sections.iter_mut().find(|(s, _)| *s == l.section) {
            Some((_, v)) => v.push((name, url)),
            None => sections.push((l.section.as_str(), vec![(name, url)])),
        }
    }
    if sections.is_empty() {
        return None;
    }
    let mut out = format!("# Links – {}\n", escape_text(course));
    for (section, items) in sections {
        out.push_str(&format!("\n## {}\n\n", escape_text(section)));
        for (name, url) in items {
            out.push_str(&format!("- [{name}]({})\n", escape_url(&url)));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(section: &str, name: &str, url: &str) -> JobLink {
        JobLink {
            section: section.into(),
            name: name.into(),
            url: url.into(),
        }
    }

    #[test]
    fn groups_by_section_and_escapes() {
        let md = links_markdown(
            "Analysis [1]",
            &[
                link("Woche 1", "Vorlesung *1*", "https://youtu.be/abc"),
                link("Woche 2", "Skript", "https://x.de/a b(c)"),
                link("Woche 1", "Zweites", "https://y.de/"),
            ],
        )
        .unwrap();
        assert!(md.starts_with("# Links – Analysis \\[1\\]"));
        assert!(md.contains("## Woche 1\n\n- [Vorlesung \\*1\\*](https://youtu.be/abc)\n- [Zweites](https://y.de/)"));
        assert!(md.contains("[Skript](https://x.de/a%20b%28c%29)"));
    }

    #[test]
    fn rejects_dangerous_schemes_and_empty() {
        assert!(links_markdown(
            "K",
            &[
                link("S", "x", "javascript:alert(1)"),
                link("S", "y", "file:///etc/passwd")
            ]
        )
        .is_none());
        assert!(links_markdown("K", &[]).is_none());
        let md = links_markdown("K", &[link("S", "", "https://a.de/")]).unwrap();
        assert!(md.contains("[https://a.de/](https://a.de/)"));
    }
}
