//! Der Download eines Auftrags. Die Teile:
//!
//! - `index`: Delta-Sync-Index und die Entscheidung, ob eine Datei übersprungen wird
//! - `fetch`: eine Datei laden, mit Wiederholungen und Drosselung
//! - `sink`: Ziel als Ordner oder ZIP
//! - `progress`: Fortschritt fürs Frontend
//! - `preview`, `links`: Vorschau und `Links.md`

#[cfg(test)]
mod e2e;
mod fetch;
mod index;
mod links;
mod preview;
mod progress;
mod sink;
#[cfg(test)]
mod tests;

use self::fetch::{run_task, Done, Opts, Task};
use self::index::{index_key, load_index, save_index, Index, IndexEntry};
use self::links::links_markdown;
use self::progress::{Shared, Ticker};
use self::sink::{finalize, part_path, Layout, Sink};
use crate::log;
use crate::session::{AppState, Ctx, SESSION_EXPIRED};
use crate::types::*;
use crate::util::{
    ext_of, fit_components, fmt_bytes, is_executable_ext, is_forbidden_dest, lock, redact, redact_url,
    render_dir_template, sanitize_component, sanitize_filename, unique_path, unique_rel,
};
use futures_util::StreamExt;
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use zip::ZipWriter;

const TMP_DIR: &str = ".moodle-scraper-tmp";
/// Windows erlaubt 260 Zeichen pro Pfad, etwas Reserve bleibt.
const MAX_PATH_TOTAL: usize = 240;
/// Grobe Annahme, wo ein ZIP später entpackt wird.
const ASSUMED_EXTRACT_BASE: usize = 80;

/// Alles, was ein Lauf über alle Kurse hinweg teilt.
struct Run<'a> {
    ctx: &'a Ctx,
    job: &'a DownloadJob,
    dest: &'a Path,
    tmp_dir: &'a Path,
    shared: &'a Arc<Shared>,
    cancel: &'a Arc<AtomicBool>,
    opts: Opts,
}

/// Gemeinsame Eingaben für Download und Vorschau.
struct Prep<'a> {
    job: &'a DownloadJob,
    tmp_dir: &'a Path,
    shared: Option<&'a Shared>,
    /// Zielordner anlegen (beim Download ja, bei der Vorschau nein).
    create_dirs: bool,
}

#[tauri::command]
pub async fn download_courses(
    app: AppHandle,
    state: State<'_, AppState>,
    job: DownloadJob,
) -> Result<DownloadSummary, String> {
    let ctx = state.ctx()?;
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);

    let dest = validate_destination(&job)?;
    let shared = Arc::new(Shared::new(
        job.courses.iter().map(|c| c.items.len()).sum(),
        job.courses.len(),
        job.parallel.clamp(1, 6),
    ));

    let ticker = Ticker::spawn(&app, shared.clone());
    let result = run_job(&ctx, &job, &dest, &shared, &cancel).await;
    ticker.stop().await;
    log_summary(&result);
    result
}

/// Vorschau: Was wäre neu, geändert oder unverändert? Es gehen nur HEAD-Anfragen raus.
#[tauri::command]
pub async fn preview_job(state: State<'_, AppState>, job: DownloadJob) -> Result<Vec<CoursePreview>, String> {
    let ctx = state.ctx()?;
    let dest = PathBuf::from(&job.dest_dir);
    if is_forbidden_dest(&dest) {
        return Err("Dieser Zielordner ist nicht erlaubt.".into());
    }
    Ok(preview::preview(&ctx, &job, &dest).await)
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn default_download_dir(app: AppHandle) -> String {
    app.path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map(|p| p.join("Moodle").to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Fasst das Ergebnis im Protokoll zusammen (höchstens 20 Fehlerzeilen).
fn log_summary(result: &Result<DownloadSummary, String>) {
    match result {
        Ok(sum) => {
            let (new, upd, skip, fail) = sum.results.iter().fold((0, 0, 0, 0), |a, r| {
                (
                    a.0 + r.downloaded,
                    a.1 + r.updated,
                    a.2 + r.skipped,
                    a.3 + r.failed.len(),
                )
            });
            log::log(
                if fail > 0 { "WARN" } else { "INFO" },
                &format!(
                    "Download beendet: {new} neu, {upd} aktualisiert, {skip} übersprungen, {fail} Fehler{}{}",
                    if sum.cancelled { " (abgebrochen)" } else { "" },
                    if sum.session_expired {
                        " (Session abgelaufen)"
                    } else {
                        ""
                    }
                ),
            );
            for f in sum.results.iter().flat_map(|r| r.failed.iter()).take(20) {
                log::log("WARN", &format!("  {}: {}", f.name, f.error));
            }
        }
        Err(e) => log::log("ERROR", &format!("Download fehlgeschlagen: {e}")),
    }
}

/// Prüft den Zielordner und ob der Platz reicht.
fn validate_destination(job: &DownloadJob) -> Result<PathBuf, String> {
    let dest = PathBuf::from(&job.dest_dir);
    if is_forbidden_dest(&dest) {
        return Err(
            "Dieser Zielordner ist nicht erlaubt. Bitte einen eigenen Ordner (z. B. in Downloads) wählen.".into(),
        );
    }
    fs::create_dir_all(&dest).map_err(|e| format!("Zielordner nicht erstellbar: {e}"))?;

    let needed: u64 = job
        .courses
        .iter()
        .flat_map(|c| c.items.iter())
        .filter_map(|i| i.size)
        .sum();
    if let Ok(free) = fs4::available_space(&dest) {
        // 5 % Puffer für die Verwaltungsdaten im ZIP
        if needed > 0 && needed + needed / 20 > free {
            return Err(format!(
                "Nicht genug freier Speicherplatz: benötigt ca. {}, frei {}.",
                fmt_bytes(needed),
                fmt_bytes(free)
            ));
        }
    }
    Ok(dest)
}

async fn run_job(
    ctx: &Ctx,
    job: &DownloadJob,
    dest: &Path,
    shared: &Arc<Shared>,
    cancel: &Arc<AtomicBool>,
) -> Result<DownloadSummary, String> {
    let tmp_dir = dest.join(TMP_DIR);
    if job.zip {
        fs::create_dir_all(&tmp_dir).map_err(|e| format!("Temp-Ordner nicht erstellbar: {e}"))?;
    }

    let run = Run {
        ctx,
        job,
        dest,
        tmp_dir: &tmp_dir,
        shared,
        cancel,
        opts: Opts {
            incremental: job.incremental,
            zip: job.zip,
            max_bytes: job.max_bytes,
        },
    };

    let mut index = load_index(dest);
    let mut counter = 0u64;
    let mut results = Vec::new();
    let mut cancelled = false;
    let mut error = None;

    for (ci, course) in job.courses.iter().enumerate() {
        shared.course_index.store(ci, Ordering::Relaxed);
        *lock(&shared.course_name) = course.name.clone();

        match run_course(&run, course, &mut index, &mut counter).await {
            Ok((res, was_cancelled)) => {
                save_index(dest, &index);
                results.push(res);
                if was_cancelled || cancel.load(Ordering::Relaxed) {
                    cancelled = true;
                    break;
                }
                if shared.expired.load(Ordering::Relaxed) {
                    // Die übrigen Kurse als offen melden, damit "Fortsetzen" sie mitnimmt
                    for rest in &job.courses[ci + 1..] {
                        results.push(unstarted_result(rest));
                    }
                    break;
                }
            }
            Err(e) => {
                error = Some(e);
                break;
            }
        }
    }

    if job.zip {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    match error {
        Some(e) => Err(e),
        None => Ok(DownloadSummary {
            results,
            cancelled,
            session_expired: shared.expired.load(Ordering::Relaxed),
        }),
    }
}

/// Legt die ZIP-Datei oder den Kursordner an.
fn open_sink(run: &Run, course: &JobCourse) -> Result<(Sink, PathBuf), String> {
    let cname = sanitize_component(&course.name);
    if run.job.zip {
        let p = unique_path(run.dest.join(format!("{cname}.zip")));
        let f = File::create(&p).map_err(|e| format!("ZIP nicht erstellbar: {e}"))?;
        Ok((Sink::Zip(Box::new(ZipWriter::new(f))), p))
    } else {
        let p = run.dest.join(&cname);
        fs::create_dir_all(&p).map_err(|e| format!("Kursordner nicht erstellbar: {e}"))?;
        Ok((Sink::Dir(p.clone()), p))
    }
}

/// Platzhalter für Kurse, die wegen der abgelaufenen Session gar nicht erst gestartet sind.
fn unstarted_result(course: &JobCourse) -> CourseResult {
    CourseResult {
        course_id: course.id.clone(),
        name: course.name.clone(),
        path: String::new(),
        downloaded: 0,
        updated: 0,
        skipped: 0,
        bytes: 0,
        failed: course
            .items
            .iter()
            .map(|i| failure(i, SESSION_EXPIRED.to_string()))
            .collect(),
    }
}

fn failure(item: &DownloadItem, error: String) -> Failure {
    Failure {
        name: item.filename.clone(),
        error,
        item: Some(item.clone()),
    }
}

/// Plant die Zielpfade und erzeugt pro Datei eine Aufgabe. Übersprungene oder
/// fehlerhafte Dateien werden gleich in `res` verbucht.
fn prepare_tasks(
    prep: &Prep,
    course: &JobCourse,
    layout: &Layout,
    index: &Index,
    counter: &mut u64,
    res: &mut CourseResult,
) -> Vec<Task> {
    let mut used = HashSet::new();
    let mut tasks = Vec::new();
    let count_done = || {
        if let Some(sh) = prep.shared {
            sh.files_done.fetch_add(1, Ordering::Relaxed);
        }
    };

    for item in &course.items {
        if !prep.job.allow_executables && is_executable_ext(&ext_of(&item.filename)) {
            res.skipped += 1;
            count_done();
            continue;
        }

        let mut comps = render_dir_template(
            &prep.job.dir_template,
            &item.section,
            item.section_index,
            item.folder.as_deref(),
        );
        comps.push(sanitize_filename(&item.filename));
        // Pfad kürzen, damit Windows ihn akzeptiert (beim ZIP rechnen wir mit einem typischen Zielordner)
        let base_len = match layout {
            Layout::Dir(base) => base.to_string_lossy().chars().count(),
            Layout::Zip => ASSUMED_EXTRACT_BASE,
        };
        let comps = fit_components(base_len, comps, MAX_PATH_TOTAL);
        let rel = unique_rel(&mut used, comps);
        let key = index_key(&course.id, &rel);
        let entry = index.get(&key).cloned();

        let (tmp, final_path, existing) = match layout {
            Layout::Dir(base) => {
                let mut p = base.clone();
                p.extend(&rel);
                if prep.create_dirs {
                    if let Some(parent) = p.parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            res.failed.push(failure(item, format!("Ordner nicht erstellbar: {e}")));
                            count_done();
                            continue;
                        }
                    }
                }
                let existing = fs::metadata(&p).ok().filter(|m| m.is_file()).map(|m| m.len());
                (part_path(&p), Some(p), existing)
            }
            Layout::Zip => {
                *counter += 1;
                (prep.tmp_dir.join(format!("{counter}.part")), None, None)
            }
        };

        tasks.push(Task {
            item: item.clone(),
            key,
            rel_str: rel.join("/"),
            tmp,
            final_path,
            entry,
            existing,
        });
    }
    tasks
}

/// Verbucht das Ergebnis einer Datei. `true` heißt: abgebrochen.
fn record_outcome(
    res: &mut CourseResult,
    sink: &mut Sink,
    index: &mut Index,
    task: &Task,
    outcome: Result<Done, String>,
) -> bool {
    match outcome {
        Ok(Done::Downloaded { size, etag, modified }) => match finalize(sink, task) {
            Ok(()) => {
                res.bytes += size;
                if task.entry.is_some() || task.existing.is_some() {
                    res.updated += 1;
                } else {
                    res.downloaded += 1;
                }
                index.insert(
                    task.key.clone(),
                    IndexEntry {
                        size,
                        etag,
                        modified,
                        url: redact_url(&task.item.url),
                    },
                );
            }
            Err(e) => res.failed.push(failure(&task.item, redact(&e))),
        },
        Ok(Done::Skipped) => res.skipped += 1,
        Ok(Done::Cancelled) => return true,
        Err(e) => res.failed.push(failure(&task.item, redact(&e))),
    }
    false
}

/// Schreibt `Links.md` in den Kursordner bzw. ins ZIP.
fn write_links(sink: &mut Sink, markdown: &str) -> Result<(), String> {
    use std::io::Write;
    match sink {
        Sink::Dir(base) => fs::write(base.join("Links.md"), markdown).map_err(|e| format!("Links.md: {e}")),
        Sink::Zip(z) => {
            z.start_file("Links.md", zip::write::SimpleFileOptions::default())
                .map_err(|e| format!("Links.md: {e}"))?;
            z.write_all(markdown.as_bytes()).map_err(|e| format!("Links.md: {e}"))
        }
    }
}

/// Lädt einen Kurs und liefert das Ergebnis und ob abgebrochen wurde.
async fn run_course(
    run: &Run<'_>,
    course: &JobCourse,
    index: &mut Index,
    counter: &mut u64,
) -> Result<(CourseResult, bool), String> {
    let (mut sink, out_path) = open_sink(run, course)?;
    let mut res = CourseResult {
        course_id: course.id.clone(),
        name: course.name.clone(),
        path: out_path.to_string_lossy().to_string(),
        downloaded: 0,
        updated: 0,
        skipped: 0,
        bytes: 0,
        failed: vec![],
    };

    let prep = Prep {
        job: run.job,
        tmp_dir: run.tmp_dir,
        shared: Some(run.shared),
        create_dirs: true,
    };
    let tasks = prepare_tasks(&prep, course, &sink.layout(), index, counter, &mut res);
    let parallel = run.job.parallel.clamp(1, 6);

    let mut cancelled = false;
    let mut stream = futures_util::stream::iter(tasks.into_iter().map(|t| {
        let (ctx, cancel, shared, opts) = (run.ctx.clone(), run.cancel.clone(), run.shared.clone(), run.opts);
        async move {
            let r = run_task(&ctx, &t, opts, &cancel, &shared).await;
            (t, r)
        }
    }))
    .buffer_unordered(parallel);

    while let Some((task, outcome)) = stream.next().await {
        run.shared.files_done.fetch_add(1, Ordering::Relaxed);
        cancelled |= record_outcome(&mut res, &mut sink, index, &task, outcome);
    }
    drop(stream);

    if run.job.save_links {
        if let Some(md) = links_markdown(&course.name, &course.links) {
            if let Err(e) = write_links(&mut sink, &md) {
                res.failed.push(Failure {
                    name: "Links.md".into(),
                    error: e,
                    item: None,
                });
            }
        }
    }

    if let Sink::Zip(z) = sink {
        if let Err(e) = z.finish() {
            res.failed.push(Failure {
                name: "ZIP".into(),
                error: format!("ZIP nicht finalisiert: {e}"),
                item: None,
            });
        }
    }
    Ok((res, cancelled))
}
