//! Scannt einen Kurs: Struktur holen (AJAX, sonst HTML) und die einzelnen Dateien auflösen.

use crate::parse::{
    find_embedded_file, is_logged_in, merge_extra_files, parse_assign_page, parse_course_page, parse_course_state,
    parse_folder_page, parse_url_page, RawItem,
};
use crate::scan_cache;
use crate::session::{ajax, net_err, AppState, Ctx, SESSION_EXPIRED};
use crate::types::{CourseContent, Item, LinkItem, Section};
use crate::util::{ext_of, filename_from_url, now_ts, parse_content_disposition, redact, url_allowed};
use futures_util::StreamExt;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use std::collections::HashSet;
use tauri::{AppHandle, Manager, State, Url};

pub struct Probe {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
}

pub fn filename_from_response(resp: &reqwest::Response) -> String {
    resp.headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_disposition)
        .unwrap_or_else(|| filename_from_url(resp.url()))
}

/// Versucht es zuerst mit HEAD. `None`, wenn das nicht reicht (Fehler, HTML-Seite).
async fn probe_head(ctx: &Ctx, url: &Url) -> Option<Probe> {
    let resp = ctx.head(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let is_html = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_lowercase().starts_with("text/html"));
    if is_html {
        return None;
    }
    let size = resp
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    Some(Probe {
        url: resp.url().to_string(),
        filename: filename_from_response(&resp),
        size,
    })
}

/// Folgt Weiterleitungen und eingebetteten Seiten bis zur eigentlichen Datei.
pub async fn probe(ctx: &Ctx, start: &Url) -> Result<Probe, String> {
    let mut target = start.clone();
    for round in 0..2 {
        if !url_allowed(&ctx.base, &target) {
            return Err("Externe Adresse nicht erlaubt.".into());
        }
        if round == 0 {
            if let Some(p) = probe_head(ctx, &target).await {
                return Ok(p);
            }
        }
        let resp = ctx.get(&target).send().await.map_err(net_err)?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status().as_u16()));
        }
        let is_html = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().starts_with("text/html"))
            .unwrap_or(false);
        if is_html {
            let final_url = resp.url().clone();
            let html = resp.text().await.map_err(|e| e.to_string())?;
            if !is_logged_in(&html) && final_url.path().contains("login") {
                return Err(SESSION_EXPIRED.into());
            }
            match find_embedded_file(&html, &final_url) {
                Some(u) => {
                    target = u;
                    continue;
                }
                None => return Err("Keine Datei (nur eine Webseite).".into()),
            }
        }
        return Ok(Probe {
            url: resp.url().to_string(),
            filename: filename_from_response(&resp),
            size: resp.content_length(),
        });
    }
    Err("Datei nicht auflösbar.".into())
}

fn make_item(
    id: String,
    name: String,
    url: String,
    filename: String,
    size: Option<u64>,
    kind: &str,
    folder: Option<String>,
) -> Item {
    Item {
        id,
        ext: ext_of(&filename),
        name: if name.is_empty() { filename.clone() } else { name },
        filename,
        url,
        size,
        kind: kind.into(),
        folder,
    }
}

async fn fetch_page(ctx: &Ctx, url: &Url) -> Result<(Url, String), String> {
    let resp = ctx.get(url).send().await.map_err(net_err)?;
    let final_url = resp.url().clone();
    let html = resp.text().await.unwrap_or_default();
    Ok((final_url, html))
}

struct Resolved {
    sec: usize,
    items: Vec<Item>,
    links: Vec<LinkItem>,
    err: Option<String>,
}

impl Resolved {
    fn empty(sec: usize) -> Self {
        Resolved {
            sec,
            items: vec![],
            links: vec![],
            err: None,
        }
    }
    fn items(sec: usize, items: Vec<Item>) -> Self {
        Resolved {
            items,
            ..Self::empty(sec)
        }
    }
    fn link(sec: usize, link: LinkItem) -> Self {
        Resolved {
            links: vec![link],
            ..Self::empty(sec)
        }
    }
    fn error(sec: usize, err: String) -> Self {
        Resolved {
            err: Some(err),
            ..Self::empty(sec)
        }
    }
}

/// Liest das Ziel einer URL-Aktivität, ohne die externe Seite zu öffnen.
async fn resolve_link(ctx: &Ctx, url: &Url) -> Result<Option<String>, String> {
    let resp = ctx.get_no_redirect(url).send().await.map_err(net_err)?;
    if resp.status().is_redirection() {
        let target = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|l| url.join(l).ok());
        return Ok(match target {
            Some(t) if url_allowed(&ctx.base, &t) => {
                // Weiterleitung zur Anmeldung heißt: Session abgelaufen. Andere interne Seiten ignorieren wir
                if t.path().contains("login") {
                    return Err(SESSION_EXPIRED.into());
                }
                None
            }
            Some(t) if matches!(t.scheme(), "http" | "https") => Some(t.to_string()),
            _ => None,
        });
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(parse_url_page(&html))
}

async fn resolve_raw(ctx: Ctx, sec: usize, raw: RawItem) -> Resolved {
    let Ok(url) = Url::parse(&raw.url) else {
        return Resolved::empty(sec);
    };
    if !url_allowed(&ctx.base, &url) {
        return Resolved::empty(sec); // externe Links werden ignoriert
    }
    match raw.kind.as_str() {
        "folder" => {
            let (final_url, html) = match fetch_page(&ctx, &url).await {
                Ok(x) => x,
                Err(e) => return Resolved::error(sec, e),
            };
            let items = parse_folder_page(&html, &final_url)
                .into_iter()
                .filter(|(u, _)| Url::parse(u).is_ok_and(|x| url_allowed(&ctx.base, &x)))
                .map(|(u, dirs)| {
                    let filename = Url::parse(&u).map(|x| filename_from_url(&x)).unwrap_or_default();
                    let mut folder = raw.name.clone();
                    for d in dirs {
                        folder.push('/');
                        folder.push_str(&d);
                    }
                    make_item(String::new(), String::new(), u, filename, None, "folder", Some(folder))
                })
                .collect();
            Resolved::items(sec, items)
        }
        "assign" => {
            let (final_url, html) = match fetch_page(&ctx, &url).await {
                Ok(x) => x,
                Err(e) => return Resolved::error(sec, e),
            };
            let items = parse_assign_page(&html, &final_url)
                .into_iter()
                .filter(|u| Url::parse(u).is_ok_and(|x| url_allowed(&ctx.base, &x)))
                .map(|u| {
                    let filename = Url::parse(&u).map(|x| filename_from_url(&x)).unwrap_or_default();
                    make_item(
                        raw.id.clone(),
                        String::new(),
                        u,
                        filename,
                        None,
                        "assign",
                        Some(raw.name.clone()),
                    )
                })
                .collect();
            Resolved::items(sec, items)
        }
        "url" => match resolve_link(&ctx, &url).await {
            Ok(Some(target)) => Resolved::link(
                sec,
                LinkItem {
                    name: raw.name,
                    url: target,
                },
            ),
            Ok(None) => Resolved::empty(sec),
            Err(e) if e == SESSION_EXPIRED => Resolved::error(sec, e),
            Err(e) => Resolved::error(sec, redact(&format!("{}: {}", raw.name, e))),
        },
        _ => match probe(&ctx, &url).await {
            Ok(p) => {
                let name = if raw.kind == "resource" {
                    raw.name.clone()
                } else {
                    String::new()
                };
                Resolved::items(
                    sec,
                    vec![make_item(raw.id, name, p.url, p.filename, p.size, &raw.kind, None)],
                )
            }
            Err(e) if e == SESSION_EXPIRED => Resolved::error(sec, e),
            Err(e) => Resolved::error(sec, redact(&format!("{}: {}", raw.name, e))),
        },
    }
}

/// Scannt einen Kurs. Solange der Cache frisch ist, wird er benutzt (außer bei `refresh`).
#[tauri::command]
pub async fn scan_course(
    app: AppHandle,
    state: State<'_, AppState>,
    course_id: String,
    refresh: Option<bool>,
) -> Result<CourseContent, String> {
    if course_id.is_empty() || !course_id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Ungültige Kurs-ID.".into());
    }
    let ctx = state.ctx()?;
    let host = ctx.base.host_str().unwrap_or_default().to_string();
    let cache_dir = app.path().app_cache_dir().ok();

    if refresh != Some(true) {
        if let Some(c) = cache_dir
            .as_ref()
            .and_then(|d| scan_cache::load(d, &host, &course_id, now_ts()))
        {
            return Ok(c);
        }
    }
    let content = scan_course_inner(&ctx, &course_id).await?;
    if let Some(d) = &cache_dir {
        scan_cache::store(d, &host, &course_id, &content, now_ts());
    }
    Ok(content)
}

#[tauri::command]
pub fn clear_scan_cache(app: AppHandle) {
    if let Ok(d) = app.path().app_cache_dir() {
        scan_cache::clear(&d);
    }
}

/// Der Scan ohne Cache, getrennt, damit die Tests ihn direkt aufrufen können.
pub async fn scan_course_inner(ctx: &Ctx, course_id: &str) -> Result<CourseContent, String> {
    let url = ctx.join(&format!("course/view.php?id={course_id}"))?;
    let resp = ctx.get(&url).send().await.map_err(net_err)?;
    if !resp.status().is_success() {
        return Err(format!("Kurs nicht erreichbar (HTTP {}).", resp.status().as_u16()));
    }
    let final_url = resp.url().clone();
    let html = resp.text().await.map_err(|e| e.to_string())?;
    if !is_logged_in(&html) {
        return Err(SESSION_EXPIRED.into());
    }

    let html_sections = parse_course_page(&html, &final_url);

    // Moodle 4.x liefert die Struktur per AJAX, sonst bleibt das HTML
    let ajax_sections = match ajax(
        ctx,
        "core_courseformat_get_state",
        serde_json::json!({"courseid": course_id.parse::<i64>().unwrap_or(0)}),
    )
    .await
    {
        Ok(data) => parse_course_state(&data, &ctx.base),
        Err(e) if e == SESSION_EXPIRED => return Err(e),
        Err(_) => None,
    };
    let raw_sections = match ajax_sections {
        Some(a) => merge_extra_files(a, &html_sections),
        None => html_sections,
    };

    let meta: Vec<(String, usize)> = raw_sections.iter().map(|s| (s.title.clone(), s.number)).collect();
    let mut tasks = Vec::new();
    for (i, s) in raw_sections.into_iter().enumerate() {
        for it in s.items {
            tasks.push((i, it));
        }
    }

    let results: Vec<Resolved> = futures_util::stream::iter(tasks)
        .map(|(i, it)| resolve_raw(ctx.clone(), i, it))
        .buffered(6)
        .collect()
        .await;

    if results.iter().any(|r| r.err.as_deref() == Some(SESSION_EXPIRED)) {
        return Err(SESSION_EXPIRED.into());
    }

    let mut sections: Vec<Section> = meta
        .into_iter()
        .map(|(title, number)| Section {
            title,
            number,
            items: vec![],
            links: vec![],
        })
        .collect();
    let mut seen = HashSet::new();
    for r in results {
        for it in r.items {
            if seen.insert(it.url.clone()) {
                sections[r.sec].items.push(it);
            }
        }
        for l in r.links {
            if !sections[r.sec].links.contains(&l) {
                sections[r.sec].links.push(l);
            }
        }
    }
    sections.retain(|s| !s.items.is_empty() || !s.links.is_empty());
    Ok(CourseContent {
        course_id: course_id.to_string(),
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{resp, spawn};

    fn ctx_for(addr: std::net::SocketAddr) -> Ctx {
        Ctx::new(Url::parse(&format!("http://{addr}/")).unwrap(), "MoodleSession=abc").unwrap()
    }

    #[tokio::test]
    async fn probe_follows_redirect_and_reads_headers() {
        let addr = spawn(|r| match r.path.as_str() {
            "/mod/resource/view.php?id=5" => resp(
                302,
                &[(
                    "Location",
                    "/pluginfile.php/1/mod_resource/content/1/Skript%20%C3%9C.pdf",
                )],
                b"",
            ),
            _ => resp(200, &[("Content-Type", "application/pdf")], b"%PDF-1234"),
        })
        .await;
        let ctx = ctx_for(addr);
        let url = ctx.join("mod/resource/view.php?id=5").unwrap();
        let p = probe(&ctx, &url).await.unwrap();
        assert_eq!(p.filename, "Skript Ü.pdf");
        assert_eq!(p.size, Some(9));
        assert!(p.url.contains("/pluginfile.php/"));
    }

    #[tokio::test]
    async fn probe_prefers_content_disposition() {
        let addr = spawn(|_| {
            resp(
                200,
                &[
                    ("Content-Type", "application/pdf"),
                    ("Content-Disposition", "attachment; filename=\"Echt.pdf\""),
                ],
                b"x",
            )
        })
        .await;
        let ctx = ctx_for(addr);
        let p = probe(&ctx, &ctx.join("pluginfile.php/1/a/b/zzz").unwrap())
            .await
            .unwrap();
        assert_eq!(p.filename, "Echt.pdf");
    }

    #[tokio::test]
    async fn probe_resolves_embedded_file_page() {
        let addr = spawn(|r| {
            if r.path.starts_with("/mod/resource") {
                resp(
                    200,
                    &[("Content-Type", "text/html")],
                    br#"<html><a href="/login/logout.php">x</a><object data="/pluginfile.php/9/mod_resource/content/2/y.pdf"></object></html>"#,
                )
            } else {
                resp(200, &[("Content-Type", "application/pdf")], b"PDF")
            }
        })
        .await;
        let ctx = ctx_for(addr);
        let p = probe(&ctx, &ctx.join("mod/resource/view.php?id=7").unwrap())
            .await
            .unwrap();
        assert_eq!(p.filename, "y.pdf");
    }

    #[tokio::test]
    async fn probe_detects_login_page_as_session_expired() {
        let addr = spawn(|_| {
            resp(
                200,
                &[("Content-Type", "text/html")],
                b"<form action=\"/login/index.php\"></form>",
            )
        })
        .await;
        let ctx = ctx_for(addr);
        let err = probe(&ctx, &ctx.join("login/index.php").unwrap()).await.err().unwrap();
        assert!(err == SESSION_EXPIRED || err.contains("Webseite"), "{err}");
    }

    #[tokio::test]
    async fn probe_refuses_external_hosts() {
        let addr = spawn(|_| resp(200, &[], b"x")).await;
        let ctx = ctx_for(addr);
        let ext = Url::parse("http://example.org/pluginfile.php/1/x.pdf").unwrap();
        assert!(probe(&ctx, &ext).await.err().unwrap().contains("nicht erlaubt"));
    }

    #[tokio::test]
    async fn probe_uses_head_and_skips_get_for_plain_files() {
        let gets = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let g = gets.clone();
        let addr = spawn(move |r| {
            if r.method == "GET" {
                g.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            resp(
                200,
                &[
                    ("Content-Type", "application/pdf"),
                    ("Content-Disposition", "attachment; filename=\"H.pdf\""),
                ],
                b"123456",
            )
        })
        .await;
        let ctx = ctx_for(addr);
        let p = probe(&ctx, &ctx.join("pluginfile.php/1/a/b/x").unwrap()).await.unwrap();
        assert_eq!(p.filename, "H.pdf");
        assert_eq!(p.size, Some(6));
        assert_eq!(gets.load(std::sync::atomic::Ordering::SeqCst), 0, "HEAD sollte genügen");
    }

    #[tokio::test]
    async fn probe_falls_back_to_get_when_head_is_rejected() {
        let addr = spawn(|r| {
            if r.method == "HEAD" {
                resp(405, &[], b"")
            } else {
                resp(200, &[("Content-Type", "application/pdf")], b"PDFDATA")
            }
        })
        .await;
        let ctx = ctx_for(addr);
        let p = probe(&ctx, &ctx.join("pluginfile.php/1/a/b/Datei.pdf").unwrap())
            .await
            .unwrap();
        assert_eq!(p.filename, "Datei.pdf");
        assert_eq!(p.size, Some(7));
    }
}
