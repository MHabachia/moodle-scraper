//! Ordnet die Dateien eines Auftrags ein (neu, geändert, unverändert), ohne etwas zu laden.

use super::fetch::Task;
use super::index::{decide_skip, load_index};
use super::sink::Layout;
use super::{prepare_tasks, Prep};
use crate::session::Ctx;
use crate::types::{CoursePreview, CourseResult, DownloadJob};
use crate::util::sanitize_component;
use futures_util::StreamExt;
use reqwest::header::{CONTENT_LENGTH, ETAG, LAST_MODIFIED};
use std::path::Path;
use tauri::Url;

const HEAD_PARALLEL: usize = 6;

/// Werte aus der HEAD-Antwort.
#[derive(Default, Clone)]
pub(super) struct Meta {
    pub(super) etag: Option<String>,
    pub(super) modified: Option<String>,
    pub(super) len: Option<u64>,
}

async fn head_meta(ctx: &Ctx, url: &str) -> Meta {
    let Ok(url) = Url::parse(url) else {
        return Meta::default();
    };
    if !crate::util::url_allowed(&ctx.base, &url) {
        return Meta::default();
    }
    let Ok(resp) = ctx.head(&url).send().await else {
        return Meta::default();
    };
    if !resp.status().is_success() {
        return Meta::default();
    }
    let h = |name| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(String::from);
    Meta {
        etag: h(ETAG),
        modified: h(LAST_MODIFIED),
        len: h(CONTENT_LENGTH).and_then(|v| v.parse().ok()),
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum Class {
    New,
    Changed,
    Unchanged,
    TooLarge,
}

pub(super) fn classify(job: &DownloadJob, task: &Task, meta: &Meta) -> (Class, Option<u64>) {
    let len = meta.len.or(task.item.size);
    if let (Some(max), Some(l)) = (job.max_bytes, len) {
        if l > max {
            return (Class::TooLarge, len);
        }
    }
    if decide_skip(
        job.incremental,
        job.zip,
        task.entry.as_ref(),
        task.existing,
        meta.etag.as_deref(),
        meta.modified.as_deref(),
        len,
    ) {
        return (Class::Unchanged, len);
    }
    if task.entry.is_some() || task.existing.is_some() {
        (Class::Changed, len)
    } else {
        (Class::New, len)
    }
}

pub(super) async fn preview(ctx: &Ctx, job: &DownloadJob, dest: &Path) -> Vec<CoursePreview> {
    let index = load_index(dest);
    let prep = Prep {
        job,
        tmp_dir: dest,
        shared: None,
        create_dirs: false,
    };
    let mut counter = 0u64;
    let mut out = Vec::new();

    for course in &job.courses {
        let layout = if job.zip {
            Layout::Zip
        } else {
            Layout::Dir(dest.join(sanitize_component(&course.name)))
        };
        let mut res = CourseResult {
            course_id: course.id.clone(),
            name: course.name.clone(),
            path: String::new(),
            downloaded: 0,
            updated: 0,
            skipped: 0,
            bytes: 0,
            failed: vec![],
        };
        let tasks = prepare_tasks(&prep, course, &layout, &index, &mut counter, &mut res);

        let urls: Vec<(usize, String)> = tasks.iter().enumerate().map(|(i, t)| (i, t.item.url.clone())).collect();
        let metas: Vec<(usize, Meta)> = futures_util::stream::iter(urls)
            .map(|(i, url)| {
                let ctx = ctx.clone();
                async move { (i, head_meta(&ctx, &url).await) }
            })
            .buffer_unordered(HEAD_PARALLEL)
            .collect()
            .await;

        let mut p = CoursePreview {
            course_id: course.id.clone(),
            name: course.name.clone(),
            blocked: res.skipped,
            links: if job.save_links { course.links.len() } else { 0 },
            ..Default::default()
        };
        for (i, meta) in metas {
            let (class, len) = classify(job, &tasks[i], &meta);
            match class {
                Class::Unchanged => p.unchanged += 1,
                Class::TooLarge => p.too_large += 1,
                Class::New | Class::Changed => {
                    if class == Class::New {
                        p.new_files += 1;
                    } else {
                        p.changed_files += 1;
                    }
                    match len {
                        Some(l) => p.bytes += l,
                        None => p.unknown_size += 1,
                    }
                }
            }
        }
        out.push(p);
    }
    out
}
