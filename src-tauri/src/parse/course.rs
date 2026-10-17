//! Liest Abschnitte und Aktivitäten aus der Kursseite (HTML) oder aus dem AJAX-Zustand.

use super::files::is_course_file;
use crate::util::{collapse_ws, html_unescape};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashSet;
use std::sync::LazyLock;
use tauri::Url;

static RE_MOD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/mod/([a-z0-9_]+)/").unwrap());

/// Aktivitätstypen, aus denen wir Dateien oder Links holen.
pub const SUPPORTED_MODS: [&str; 4] = ["resource", "folder", "assign", "url"];

#[derive(Debug, Clone, PartialEq)]
pub struct RawItem {
    pub id: String,
    pub name: String,
    pub kind: String, // resource | folder | assign | file
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawSection {
    pub title: String,
    pub number: usize,
    pub items: Vec<RawItem>,
}

fn element_text_without_hidden(el: ElementRef, hide: &Selector) -> String {
    let mut full = el.text().collect::<String>();
    for h in el.select(hide) {
        let t = h.text().collect::<String>();
        if !t.is_empty() {
            full = full.replacen(&t, "", 1);
        }
    }
    collapse_ws(&full)
}

pub fn parse_course_page(html: &str, page: &Url) -> Vec<RawSection> {
    let doc = Html::parse_document(html);
    let sec_sel = Selector::parse("li.section, li[id^='section-']").unwrap();
    let act_sel = Selector::parse("li.activity").unwrap();
    let a_sel = Selector::parse("a[href]").unwrap();
    let name_sel = Selector::parse(".instancename").unwrap();
    let hide_sel = Selector::parse(".accesshide, .sr-only").unwrap();
    let title_sel = Selector::parse(".sectionname, [data-for='section_title'], h3").unwrap();

    let build = |root: ElementRef, idx: usize, fallback_title: &str| -> RawSection {
        let title = root
            .select(&title_sel)
            .map(|t| collapse_ws(&t.text().collect::<String>()))
            .find(|t| !t.is_empty())
            .unwrap_or_else(|| {
                if idx == 0 {
                    fallback_title.to_string()
                } else {
                    format!("Abschnitt {idx}")
                }
            });
        let mut items = Vec::new();
        let mut taken = HashSet::new();

        for act in root.select(&act_sel) {
            let anchor = act.select(&a_sel).find(|a| {
                let h = a.value().attr("href").unwrap_or_default();
                h.contains("/mod/") && h.contains(".php")
            });
            let Some(anchor) = anchor else { continue };
            let href = anchor.value().attr("href").unwrap_or_default();
            let Some(kind) = RE_MOD.captures(href).map(|c| c[1].to_string()) else {
                continue;
            };
            if !SUPPORTED_MODS.contains(&kind.as_str()) {
                continue;
            }
            let Ok(url) = page.join(href) else { continue };
            let name = act
                .select(&name_sel)
                .next()
                .map(|n| element_text_without_hidden(n, &hide_sel))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| collapse_ws(&anchor.text().collect::<String>()));
            let id = act
                .value()
                .attr("id")
                .map(|i| i.trim_start_matches("module-").to_string())
                .unwrap_or_default();
            taken.insert(url.to_string());
            items.push(RawItem {
                id,
                name,
                kind,
                url: url.to_string(),
            });
        }

        for a in root.select(&a_sel) {
            let href = a.value().attr("href").unwrap_or_default();
            if !is_course_file(href) {
                continue;
            }
            let Ok(url) = page.join(href) else { continue };
            if !taken.insert(url.to_string()) {
                continue;
            }
            items.push(RawItem {
                id: String::new(),
                name: collapse_ws(&a.text().collect::<String>()),
                kind: "file".into(),
                url: url.to_string(),
            });
        }
        RawSection {
            title,
            number: idx,
            items,
        }
    };

    let sections: Vec<ElementRef> = doc.select(&sec_sel).collect();
    if sections.is_empty() {
        return vec![build(doc.root_element(), 0, "Kurs")];
    }
    sections
        .into_iter()
        .enumerate()
        .map(|(i, s)| build(s, i, "Allgemein"))
        .collect()
}

/// Struktur aus `core_courseformat_get_state` (Moodle 4.x). `data` ist ein JSON-String
/// oder schon ein Objekt.
pub fn parse_course_state(data: &serde_json::Value, base: &Url) -> Option<Vec<RawSection>> {
    let v: serde_json::Value = match data {
        serde_json::Value::String(s) => serde_json::from_str(s).ok()?,
        other => other.clone(),
    };
    let sections = v.get("section")?.as_array()?;
    let cms = v.get("cm")?.as_array()?;

    let id_str = |x: &serde_json::Value| -> Option<String> {
        match x {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };

    let mut out = Vec::new();
    for (i, s) in sections.iter().enumerate() {
        let number = s
            .get("number")
            .and_then(|n| n.as_u64())
            .map(|n| n as usize)
            .unwrap_or(i);
        let title = s
            .get("title")
            .and_then(|t| t.as_str())
            .map(|t| collapse_ws(&html_unescape(t)))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                if number == 0 {
                    "Allgemein".into()
                } else {
                    format!("Abschnitt {number}")
                }
            });
        let list: Vec<String> = s
            .get("cmlist")
            .and_then(|l| l.as_array())
            .map(|l| l.iter().filter_map(id_str).collect())
            .unwrap_or_default();
        let mut items = Vec::new();
        for cm_id in list {
            let Some(cm) = cms
                .iter()
                .find(|c| c.get("id").and_then(id_str).as_deref() == Some(cm_id.as_str()))
            else {
                continue;
            };
            if cm.get("uservisible").and_then(|b| b.as_bool()) == Some(false) {
                continue;
            }
            let module = cm.get("module").and_then(|m| m.as_str()).unwrap_or("");
            if !SUPPORTED_MODS.contains(&module) {
                continue;
            }
            let Some(url) = cm.get("url").and_then(|u| u.as_str()) else {
                continue;
            };
            let Ok(url) = base.join(url) else { continue };
            items.push(RawItem {
                id: cm_id.clone(),
                name: collapse_ws(&html_unescape(cm.get("name").and_then(|n| n.as_str()).unwrap_or(""))),
                kind: module.to_string(),
                url: url.to_string(),
            });
        }
        out.push(RawSection { title, number, items });
    }
    if out.iter().all(|s| s.items.is_empty()) {
        None
    } else {
        Some(out)
    }
}

/// Ergänzt die AJAX-Struktur um Dateien, die nur im HTML stehen (Kind "file").
pub fn merge_extra_files(mut base: Vec<RawSection>, html: &[RawSection]) -> Vec<RawSection> {
    let known: HashSet<String> = base
        .iter()
        .flat_map(|s| s.items.iter().map(|i| i.url.clone()))
        .collect();
    for hs in html {
        for it in hs.items.iter().filter(|i| i.kind == "file" && !known.contains(&i.url)) {
            let idx = base
                .iter()
                .position(|b| b.title.eq_ignore_ascii_case(&hs.title))
                .or_else(|| base.iter().position(|b| b.number == hs.number))
                .unwrap_or(0);
            if let Some(sec) = base.get_mut(idx) {
                sec.items.push(it.clone());
            }
        }
    }
    base
}
