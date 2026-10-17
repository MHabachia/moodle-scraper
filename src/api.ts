import {invoke} from '@tauri-apps/api/core';
import {listen, type UnlistenFn} from '@tauri-apps/api/event';
import {open, save} from '@tauri-apps/plugin-dialog';
import {getVersion} from '@tauri-apps/api/app';
import {openUrl, revealItemInDir} from '@tauri-apps/plugin-opener';
import type {
  Course,
  CourseContent,
  CoursePreview,
  Deadline,
  DownloadJob,
  DownloadSummary,
  Progress,
  SessionInfo,
} from './generated';

// Typen kommen aus dem Rust-Backend (src-tauri/src/types.rs -> src/generated).
export type {
  Course,
  CourseContent,
  CoursePreview,
  CourseResult,
  Deadline,
  DownloadItem,
  DownloadJob,
  DownloadSummary,
  Item,
  JobLink,
  Progress,
  SessionInfo,
} from './generated';

export const SESSION_EXPIRED = 'SESSION_EXPIRED';
export const LOGIN_WINDOW_CLOSED = 'LOGIN_WINDOW_CLOSED';

export const api = {
  openLogin: (moodleUrl: string) => invoke<string>('open_login', {moodleUrl}),
  checkLogin: () => invoke<SessionInfo | null>('check_login'),
  restoreSession: (moodleUrl: string) => invoke<SessionInfo | null>('restore_session', {moodleUrl}),
  setRemember: (enabled: boolean, moodleUrl?: string) => invoke<void>('set_remember', {enabled, moodleUrl}),
  logout: () => invoke<void>('logout'),
  listCourses: () => invoke<Course[]>('list_courses'),
  scanCourse: (courseId: string, refresh = false) => invoke<CourseContent>('scan_course', {courseId, refresh}),
  clearScanCache: () => invoke<void>('clear_scan_cache'),
  previewJob: (job: DownloadJob) => invoke<CoursePreview[]>('preview_job', {job}),
  download: (job: DownloadJob) => invoke<DownloadSummary>('download_courses', {job}),
  cancelDownload: () => invoke<void>('cancel_download'),
  defaultDir: () => invoke<string>('default_download_dir'),
  listDeadlines: () => invoke<Deadline[]>('list_deadlines'),
  exportIcs: (path: string, deadlines: Deadline[]) => invoke<void>('export_ics', {path, deadlines}),
  exportDiagnostics: (path: string) => invoke<void>('export_diagnostics', {path}),
  setBackground: (enabled: boolean) => invoke<void>('set_background', {enabled}),
  pickFolder: async (defaultPath?: string) => {
    const res = await open({directory: true, multiple: false, defaultPath, title: 'Zielordner wählen'});
    return typeof res === 'string' ? res : null;
  },
  pickSavePath: async (defaultPath: string, ext: string) => {
    const res = await save({defaultPath, filters: [{name: ext.toUpperCase(), extensions: [ext]}]});
    return typeof res === 'string' ? res : null;
  },
  reveal: (path: string) => revealItemInDir(path),
  openUrl: (url: string) => openUrl(url),
  appVersion: () => getVersion(),
  onProgress: (cb: (p: Progress) => void): Promise<UnlistenFn> =>
    listen<Progress>('download-progress', (e) => cb(e.payload)),
};

export function errorMessage(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
}
