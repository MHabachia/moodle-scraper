import {useCallback, useEffect, useRef, useState} from 'react';
import {
  api,
  errorMessage,
  LOGIN_WINDOW_CLOSED,
  SESSION_EXPIRED,
  type Course,
  type CourseContent,
  type CoursePreview,
  type DownloadSummary,
  type Progress,
} from '../api';
import type {Translator} from '../i18n';
import {makeJob, retryCourses, type JobCourses} from '../lib/job';
import type {Phase} from '../types';
import {buildCourseItems, buildCourseLinks, courseFolderName, type Settings} from '../utils';

const SCAN_WORKERS = 3;
const RELOGIN_POLL_MS = 2500;
const RELOGIN_MAX_POLLS = 120; // ~5 Minuten

export type ReloginState = 'idle' | 'waiting' | 'failed';

interface Options {
  settings: Settings;
  t: Translator;
  cache: Record<string, CourseContent>;
  excluded: Record<string, string[]>;
  onScanned: (courseId: string, content: CourseContent) => void;
  /** Fehler behandeln (z. B. abgelaufene Session). */
  onError: (e: unknown) => void;
  /** Hinweis anzeigen (kein Fehlerfall der Session). */
  onMessage: (msg: string) => void;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * Ablauf: Kurse analysieren (parallel) → optional Vorschau → herunterladen → Ergebnis.
 * Dazu Abbruch, Wiederholen, Fortsetzen nach erneutem Login und stiller Hintergrund-Sync.
 */
export function useDownload(o: Options) {
  const [phase, setPhase] = useState<Phase>('idle');
  const [scanInfo, setScanInfo] = useState({index: 0, total: 0, name: '', preparing: false});
  const [progress, setProgress] = useState<Progress | null>(null);
  const [summary, setSummary] = useState<DownloadSummary | null>(null);
  const [previews, setPreviews] = useState<CoursePreview[]>([]);
  const [relogin, setRelogin] = useState<ReloginState>('idle');
  const cancelRef = useRef(false);
  const pendingRef = useRef<JobCourses | null>(null);
  const opts = useRef(o);
  opts.current = o;

  useEffect(() => {
    let un: (() => void) | undefined;
    api.onProgress(setProgress).then((u) => (un = u));
    return () => un?.();
  }, []);

  const runJob = useCallback(async (courses: JobCourses) => {
    setPhase('downloading');
    setProgress(null);
    const res = await api.download(makeJob(opts.current.settings, courses));
    setSummary(res);
    setPhase('done');
  }, []);

  /** Analysiert die Kurse mit mehreren Workern; Reihenfolge der Kurse bleibt erhalten. */
  const scanAll = useCallback(async (chosen: Course[], refresh: boolean): Promise<JobCourses> => {
    const slots: (JobCourses[number] | null)[] = chosen.map(() => null);
    let next = 0;
    let finished = 0;
    setScanInfo({index: 0, total: chosen.length, name: chosen[0].name, preparing: false});

    const worker = async () => {
      while (!cancelRef.current) {
        const i = next++;
        if (i >= chosen.length) return;
        const k = chosen[i];
        const {settings, cache, excluded, onScanned} = opts.current;
        let content = refresh ? undefined : cache[k.id];
        if (!content) {
          try {
            content = await api.scanCourse(k.id, refresh);
            onScanned(k.id, content);
          } catch (e) {
            if (errorMessage(e) === SESSION_EXPIRED) {
              cancelRef.current = true;
              throw e;
            }
            finished++; // Kurs nicht lesbar -> überspringen
            setScanInfo({index: finished, total: chosen.length, name: k.name, preparing: false});
            continue;
          }
        }
        const items = buildCourseItems(content, {
          excluded: new Set(excluded[k.id] ?? []),
          types: settings.types,
          maxSizeMb: settings.maxSizeMb,
        });
        const links = settings.saveLinks ? buildCourseLinks(content) : [];
        if (items.length || links.length) {
          slots[i] = {id: k.id, name: courseFolderName(k, settings.courseNaming), items, links};
        }
        finished++;
        setScanInfo({index: finished, total: chosen.length, name: k.name, preparing: false});
      }
    };

    await Promise.all(Array.from({length: Math.min(SCAN_WORKERS, chosen.length)}, worker));
    return slots.filter((x): x is JobCourses[number] => x !== null);
  }, []);

  const start = useCallback(
    async (chosen: Course[], refresh = false) => {
      if (!chosen.length) return;
      cancelRef.current = false;
      setSummary(null);
      setProgress(null);
      setPreviews([]);
      setPhase('scanning');
      try {
        const job = await scanAll(chosen, refresh);
        if (cancelRef.current) {
          setPhase('idle');
          return;
        }
        if (!job.length) {
          opts.current.onMessage(opts.current.t('msg.noFiles'));
          setPhase('idle');
          return;
        }
        if (opts.current.settings.preview) {
          setScanInfo((s) => ({...s, preparing: true}));
          const result = await api.previewJob(makeJob(opts.current.settings, job));
          if (cancelRef.current) {
            setPhase('idle');
            return;
          }
          pendingRef.current = job;
          setPreviews(result);
          setPhase('preview');
          return;
        }
        await runJob(job);
      } catch (e) {
        setPhase('idle');
        opts.current.onError(e);
      }
    },
    [scanAll, runJob],
  );

  /** Vorschau bestätigt: Download starten. */
  const confirm = useCallback(async () => {
    const job = pendingRef.current;
    if (!job) return;
    pendingRef.current = null;
    try {
      await runJob(job);
    } catch (e) {
      setPhase('idle');
      opts.current.onError(e);
    }
  }, [runJob]);

  const retry = useCallback(async () => {
    if (!summary) return;
    const courses = retryCourses(summary);
    if (!courses.length) return;
    setSummary(null);
    try {
      await runJob(courses);
    } catch (e) {
      setPhase('idle');
      opts.current.onError(e);
    }
  }, [summary, runJob]);

  /** Session abgelaufen: Login-Fenster öffnen, auf Anmeldung warten, dann den Rest laden. */
  const reloginAndResume = useCallback(async () => {
    setRelogin('waiting');
    let ok = false;
    try {
      await api.openLogin(opts.current.settings.moodleUrl);
      for (let i = 0; i < RELOGIN_MAX_POLLS && !ok; i++) {
        await sleep(RELOGIN_POLL_MS);
        try {
          ok = (await api.checkLogin()) !== null;
        } catch (e) {
          if (errorMessage(e) === LOGIN_WINDOW_CLOSED) break;
        }
      }
    } catch {
      ok = false;
    }
    setRelogin(ok ? 'idle' : 'failed');
    if (ok) await retry();
  }, [retry]);

  /** Stiller Lauf ohne Dialoge (Auto-Sync). `null`, wenn gerade etwas anderes läuft oder nichts zu tun ist. */
  const sync = useCallback(
    async (chosen: Course[]): Promise<DownloadSummary | null> => {
      if (!chosen.length || phase !== 'idle') return null;
      cancelRef.current = false;
      const job = await scanAll(chosen, true);
      if (!job.length) return null;
      return api.download(makeJob(opts.current.settings, job));
    },
    [phase, scanAll],
  );

  const cancel = useCallback(async () => {
    cancelRef.current = true;
    await api.cancelDownload().catch(() => {});
  }, []);

  const close = useCallback(() => {
    pendingRef.current = null;
    setPhase('idle');
    setSummary(null);
    setRelogin('idle');
  }, []);

  return {
    phase,
    scanInfo,
    progress,
    summary,
    previews,
    relogin,
    start,
    confirm,
    retry,
    reloginAndResume,
    sync,
    cancel,
    close,
  };
}
