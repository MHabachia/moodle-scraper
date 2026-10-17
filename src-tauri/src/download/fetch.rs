//! Lädt eine einzelne Datei: Anfrage, Streaming auf die Platte, Wiederholungen mit Wartezeit.

use super::index::{decide_skip, IndexEntry};
use super::progress::Shared;
use crate::parse::is_logged_in;
use crate::session::{net_err, Ctx, SESSION_EXPIRED};
use crate::types::DownloadItem;
use crate::util::{ext_of, lock, url_allowed};
use futures_util::StreamExt;
use reqwest::header::{CONTENT_TYPE, ETAG, LAST_MODIFIED, RETRY_AFTER};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Url;

const MAX_ATTEMPTS: u64 = 3;
const BACKOFF_MS: u64 = if cfg!(test) { 10 } else { 700 };

#[derive(Clone, Copy)]
pub(super) struct Opts {
    pub(super) incremental: bool,
    pub(super) zip: bool,
    pub(super) max_bytes: Option<u64>,
}

pub(super) enum Done {
    Downloaded {
        size: u64,
        etag: Option<String>,
        modified: Option<String>,
    },
    Skipped,
    Cancelled,
}

pub(super) enum AttemptErr {
    /// `Retry(Meldung, Retry-After in Sekunden, Server überlastet?)`
    Retry(String, Option<u64>, bool),
    Fatal(String),
}

pub(super) struct Task {
    pub(super) item: DownloadItem,
    pub(super) key: String,
    pub(super) rel_str: String,
    pub(super) tmp: PathBuf,
    pub(super) final_path: Option<PathBuf>,
    pub(super) entry: Option<IndexEntry>,
    pub(super) existing: Option<u64>,
}

/// Liest den Anfang einer Antwort, um zu sehen, was für eine Seite geliefert wurde.
async fn read_prefix(mut resp: reqwest::Response, limit: usize) -> String {
    let mut buf = Vec::new();
    while let Ok(Some(chunk)) = resp.chunk().await {
        buf.extend_from_slice(&chunk);
        if buf.len() >= limit {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

pub(super) async fn attempt(
    ctx: &Ctx,
    task: &Task,
    opts: Opts,
    cancel: &AtomicBool,
    shared: &Shared,
) -> Result<Done, AttemptErr> {
    let url = Url::parse(&task.item.url).map_err(|_| AttemptErr::Fatal("Ungültige Datei-URL".into()))?;
    if !url_allowed(&ctx.base, &url) {
        return Err(AttemptErr::Fatal(
            "Adresse gehört nicht zur Moodle-Instanz (blockiert)".into(),
        ));
    }
    let resp = ctx
        .get(&url)
        .send()
        .await
        .map_err(|e| AttemptErr::Retry(net_err(e), None, false))?;
    let status = resp.status();
    if status.as_u16() == 429 || status.is_server_error() {
        let wait = resp
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        return Err(AttemptErr::Retry(format!("HTTP {}", status.as_u16()), wait, true));
    }
    if !status.is_success() {
        return Err(AttemptErr::Fatal(format!("HTTP {}", status.as_u16())));
    }

    let header = |h| {
        resp.headers()
            .get(h)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    let is_html = header(CONTENT_TYPE).is_some_and(|v| v.to_lowercase().starts_with("text/html"));
    let etag = header(ETAG);
    let modified = header(LAST_MODIFIED);
    let len = resp.content_length();

    let wants_html = matches!(ext_of(&task.item.filename).as_str(), "html" | "htm");
    if is_html && !wants_html {
        // Login-Seite (Session weg) oder irgendeine andere Webseite?
        let page = read_prefix(resp, 256 * 1024).await;
        return Err(AttemptErr::Fatal(if is_logged_in(&page) {
            "Server lieferte eine Webseite statt der Datei.".into()
        } else {
            SESSION_EXPIRED.into()
        }));
    }

    if let (Some(max), Some(l)) = (opts.max_bytes, len) {
        if l > max {
            return Ok(Done::Skipped);
        }
    }
    if decide_skip(
        opts.incremental,
        opts.zip,
        task.entry.as_ref(),
        task.existing,
        etag.as_deref(),
        modified.as_deref(),
        len,
    ) {
        return Ok(Done::Skipped);
    }

    let mut file = File::create(&task.tmp).map_err(|e| AttemptErr::Fatal(format!("Datei nicht schreibbar: {e}")))?;
    let mut stream = resp.bytes_stream();
    let mut written: u64 = 0;
    let rollback = |written: u64| {
        shared.bytes.fetch_sub(written, Ordering::Relaxed);
        let _ = fs::remove_file(&task.tmp);
    };

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            rollback(written);
            return Ok(Done::Cancelled);
        }
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                drop(file);
                rollback(written);
                return Err(AttemptErr::Retry(net_err(e), None, false));
            }
        };
        if let Err(e) = file.write_all(&chunk) {
            drop(file);
            rollback(written);
            return Err(AttemptErr::Fatal(format!("Schreibfehler: {e}")));
        }
        written += chunk.len() as u64;
        shared.bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    drop(file);

    if let Some(l) = len {
        if l != written {
            rollback(written);
            return Err(AttemptErr::Retry(
                format!("Unvollständig ({written} von {l} Bytes)"),
                None,
                false,
            ));
        }
    }
    Ok(Done::Downloaded {
        size: written,
        etag,
        modified,
    })
}

pub(super) async fn run_task(
    ctx: &Ctx,
    task: &Task,
    opts: Opts,
    cancel: &AtomicBool,
    shared: &Shared,
) -> Result<Done, String> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(Done::Cancelled);
    }
    if shared.expired.load(Ordering::Relaxed) {
        return Err(SESSION_EXPIRED.into());
    }
    // Wartet auf einen freien Platz; bei Überlast gibt es weniger davon
    let _permit = shared.permits.acquire().await.ok();
    if cancel.load(Ordering::Relaxed) {
        return Ok(Done::Cancelled);
    }
    if shared.expired.load(Ordering::Relaxed) {
        return Err(SESSION_EXPIRED.into());
    }

    lock(&shared.active).push(task.item.filename.clone());
    let mut result: Result<Done, String> = Err("Unbekannter Fehler".to_string());
    for n in 0..MAX_ATTEMPTS {
        match attempt(ctx, task, opts, cancel, shared).await {
            Ok(d) => {
                result = Ok(d);
                break;
            }
            Err(AttemptErr::Fatal(e)) => {
                if e == SESSION_EXPIRED {
                    shared.expired.store(true, Ordering::Relaxed);
                }
                result = Err(e);
                break;
            }
            Err(AttemptErr::Retry(e, hint, overloaded)) => {
                if overloaded {
                    shared.throttle();
                }
                result = Err(format!("{e} (nach {} Versuchen)", n + 1));
                if cancel.load(Ordering::Relaxed) {
                    result = Ok(Done::Cancelled);
                    break;
                }
                if n + 1 < MAX_ATTEMPTS {
                    let backoff = Duration::from_millis(BACKOFF_MS * (n + 1) * (n + 1));
                    let wait = hint.map_or(backoff, |s| Duration::from_secs(s.min(30)).max(backoff));
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }
    {
        let mut a = lock(&shared.active);
        if let Some(pos) = a.iter().position(|f| f == &task.item.filename) {
            a.remove(pos);
        }
    }
    result
}
