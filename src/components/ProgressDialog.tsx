import {useRef} from 'react';
import {AlertTriangle, CheckCircle2, FolderOpen, Loader2, RefreshCw} from 'lucide-react';
import {api, SESSION_EXPIRED, type CourseResult, type DownloadSummary, type Progress} from '../api';
import type {ReloginState} from '../hooks/useDownload';
import {useT} from '../i18n';
import type {Phase} from '../types';
import {Bar, btnGhost, btnPrimary, Modal} from '../ui';
import {formatBytes} from '../utils';

export interface SpeedSample {
  t: number;
  bytes: number;
  speed: number;
}

/** Geglättete Geschwindigkeit (Bytes/s) aus aufeinanderfolgenden Fortschrittswerten. */
export function updateSpeed(prev: SpeedSample, bytesDone: number, now: number): SpeedSample {
  if (prev.t === 0 || bytesDone < prev.bytes) return {t: now, bytes: bytesDone, speed: 0};
  if (now - prev.t < 700) return prev;
  const inst = ((bytesDone - prev.bytes) / (now - prev.t)) * 1000;
  return {t: now, bytes: bytesDone, speed: prev.speed ? prev.speed * 0.6 + inst * 0.4 : inst};
}

interface Props {
  phase: Phase;
  scan: {index: number; total: number; name: string; preparing: boolean};
  progress: Progress | null;
  summary: DownloadSummary | null;
  relogin: ReloginState;
  onRetry: () => void;
  onRelogin: () => void;
  onCancel: () => void;
  onClose: () => void;
}

export function ProgressDialog(props: Props) {
  const t = useT();
  const {phase, progress, summary} = props;
  const sample = useRef<SpeedSample>({t: 0, bytes: 0, speed: 0});
  if (progress) sample.current = updateSpeed(sample.current, progress.bytesDone, Date.now());

  if (phase === 'done' && summary) {
    const sum = (f: (r: CourseResult) => number) => summary.results.reduce((a, r) => a + f(r), 0);
    const failed = summary.results.flatMap((r) => r.failed.map((f) => ({...f, course: r.name})));
    const errText = (e: string) => (e === SESSION_EXPIRED ? t('progress.expiredItem') : e);
    const report = () =>
      navigator.clipboard
        ?.writeText(failed.map((f) => `${f.course} / ${f.name}: ${errText(f.error)}`).join('\n'))
        .catch(() => {});
    return (
      <Modal
        title={summary.cancelled ? t('progress.titleCancelled') : t('progress.titleDone')}
        onClose={props.onClose}
        wide
      >
        <div className="flex-1 space-y-4 overflow-y-auto p-5">
          <div className="flex items-center gap-3">
            <CheckCircle2 className={`h-9 w-9 ${summary.cancelled ? 'text-amber-500' : 'text-emerald-500'}`} />
            <div>
              <div className="text-lg font-semibold">
                {t('progress.summary', {
                  new: sum((r) => r.downloaded),
                  updated: sum((r) => r.updated),
                  size: formatBytes(sum((r) => r.bytes)),
                })}
              </div>
              <div className="text-sm text-slate-500">
                {t('progress.summary2', {skipped: sum((r) => r.skipped), failed: failed.length})}
              </div>
            </div>
          </div>

          {summary.sessionExpired && (
            <div className="flex items-start gap-3 rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm dark:border-amber-700 dark:bg-amber-950/40">
              <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-600" />
              <div className="flex-1">
                <div className="font-semibold">{t('progress.expiredTitle')}</div>
                <p className="text-slate-600 dark:text-slate-300">{t('progress.expiredText')}</p>
                {props.relogin === 'failed' && <p className="mt-1 text-red-600">{t('progress.reloginFailed')}</p>}
                <button className={`${btnPrimary} mt-2`} onClick={props.onRelogin} disabled={props.relogin === 'waiting'}>
                  {props.relogin === 'waiting' ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" /> {t('progress.reloginWaiting')}
                    </>
                  ) : (
                    t('progress.relogin')
                  )}
                </button>
              </div>
            </div>
          )}

          <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-700 dark:border-slate-700">
            {summary.results.map((r) => (
              <li key={r.courseId} className="flex items-center gap-3 px-3 py-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{r.name}</div>
                  <div className="text-xs text-slate-500">
                    {t('progress.courseLine', {
                      new: r.downloaded,
                      updated: r.updated,
                      skipped: r.skipped,
                      failed: r.failed.length,
                    })}
                  </div>
                </div>
                {r.path && (
                  <button className={btnGhost} onClick={() => api.reveal(r.path).catch(() => {})}>
                    <FolderOpen className="h-4 w-4" /> {t('common.open')}
                  </button>
                )}
              </li>
            ))}
          </ul>
          {failed.length > 0 && (
            <details className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm dark:border-red-900 dark:bg-red-950/40">
              <summary className="cursor-pointer font-medium text-red-700 dark:text-red-300">
                {t('progress.errors', {n: failed.length})}
              </summary>
              <ul className="mt-2 space-y-1 text-xs">
                {failed.map((f, i) => (
                  <li key={i}>
                    <b>{f.course}</b> – {f.name}: {errText(f.error)}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
        <div className="flex justify-end gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-700">
          {failed.length > 0 && (
            <>
              <button className={btnGhost} onClick={report}>
                {t('progress.copyReport')}
              </button>
              {!summary.sessionExpired && (
                <button className={btnGhost} onClick={props.onRetry}>
                  <RefreshCw className="h-4 w-4" /> {t('progress.retry')}
                </button>
              )}
            </>
          )}
          <button className={btnPrimary} onClick={props.onClose}>
            {t('common.close')}
          </button>
        </div>
      </Modal>
    );
  }

  const pct = progress && progress.filesTotal ? Math.round((progress.filesDone / progress.filesTotal) * 100) : 0;
  const speed = sample.current.speed;

  return (
    <Modal title={phase === 'scanning' ? t('progress.scanning') : t('progress.downloading')}>
      <div className="space-y-4 p-6">
        {phase === 'scanning' ? (
          <>
            <div className="flex items-center gap-3">
              <Loader2 className="h-5 w-5 animate-spin text-indigo-500" />
              <span className="text-sm">
                {props.scan.preparing
                  ? t('progress.preparing')
                  : t('progress.scanProgress', {done: props.scan.index, total: props.scan.total})}
              </span>
            </div>
            <div className="truncate text-sm text-slate-500">{props.scan.name}</div>
            <Bar value={props.scan.total ? (props.scan.index / props.scan.total) * 100 : 0} />
          </>
        ) : (
          <>
            <div className="flex items-baseline justify-between text-sm">
              <span className="font-medium">
                {progress
                  ? t('progress.files', {done: progress.filesDone, total: progress.filesTotal})
                  : t('progress.starting')}
              </span>
              <span className="text-slate-500">
                {progress ? formatBytes(progress.bytesDone) : ''}
                {speed > 0 ? ` · ${formatBytes(Math.round(speed))}/s` : ''}
              </span>
            </div>
            <Bar value={pct} />
            {progress && (
              <div className="space-y-1">
                <div className="truncate text-xs text-slate-500">
                  {t('progress.course', {
                    n: progress.courseIndex + 1,
                    total: progress.courseCount,
                    name: progress.courseName,
                  })}
                </div>
                {progress.active.slice(0, 4).map((f, i) => (
                  <div key={`${f}-${i}`} className="flex items-center gap-2 truncate text-sm">
                    <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-indigo-500" /> {f}
                  </div>
                ))}
              </div>
            )}
          </>
        )}
        <div className="flex justify-end pt-2">
          <button className={btnGhost} onClick={props.onCancel}>
            {t('common.cancel')}
          </button>
        </div>
      </div>
    </Modal>
  );
}
