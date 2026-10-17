//! Geteilter Fortschritt und das Event, das ihn ans Frontend schickt.

use crate::types::Progress;
use crate::util::lock;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

pub(super) struct Shared {
    pub(super) files_done: AtomicUsize,
    pub(super) bytes: AtomicU64,
    pub(super) active: Mutex<Vec<String>>,
    pub(super) course_index: AtomicUsize,
    pub(super) course_name: Mutex<String>,
    pub(super) files_total: usize,
    pub(super) course_count: usize,
    /// Session ist abgelaufen: weitere Dateien werden nicht mehr versucht.
    pub(super) expired: AtomicBool,
    /// Begrenzt gleichzeitige Downloads; bei 429/503 wird die Zahl halbiert.
    pub(super) permits: Semaphore,
    capacity: AtomicUsize,
}

impl Shared {
    pub(super) fn new(files_total: usize, course_count: usize, parallel: usize) -> Self {
        let parallel = parallel.max(1);
        Shared {
            files_done: AtomicUsize::new(0),
            bytes: AtomicU64::new(0),
            active: Mutex::new(vec![]),
            course_index: AtomicUsize::new(0),
            course_name: Mutex::new(String::new()),
            files_total,
            course_count,
            expired: AtomicBool::new(false),
            permits: Semaphore::new(parallel),
            capacity: AtomicUsize::new(parallel),
        }
    }

    /// Halbiert die Parallelität (mindestens 1). `true`, wenn sie dadurch gesunken ist.
    pub(super) fn throttle(&self) -> bool {
        let cap = self.capacity.load(Ordering::Relaxed);
        if cap <= 1 {
            return false;
        }
        let new = (cap / 2).max(1);
        if self
            .capacity
            .compare_exchange(cap, new, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.permits.forget_permits(cap - new);
            return true;
        }
        false
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    pub(super) fn snapshot(&self) -> Progress {
        Progress {
            course_index: self.course_index.load(Ordering::Relaxed),
            course_count: self.course_count,
            course_name: lock(&self.course_name).clone(),
            files_done: self.files_done.load(Ordering::Relaxed),
            files_total: self.files_total,
            bytes_done: self.bytes.load(Ordering::Relaxed),
            active: lock(&self.active).clone(),
        }
    }
}

/// Schickt alle 250 ms den Fortschritt, bis `stop()` kommt.
pub(super) struct Ticker {
    stop: Arc<AtomicBool>,
    handle: tauri::async_runtime::JoinHandle<()>,
}

impl Ticker {
    pub(super) fn spawn(app: &AppHandle, shared: Arc<Shared>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let (app, stop) = (app.clone(), stop.clone());
            tauri::async_runtime::spawn(async move {
                while !stop.load(Ordering::Relaxed) {
                    let _ = app.emit("download-progress", shared.snapshot());
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                let _ = app.emit("download-progress", shared.snapshot());
            })
        };
        Ticker { stop, handle }
    }

    pub(super) async fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.await;
    }
}
