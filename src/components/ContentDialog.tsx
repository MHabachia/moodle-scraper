import {useEffect, useState} from 'react';
import {ExternalLink, Loader2} from 'lucide-react';
import {api, errorMessage, SESSION_EXPIRED, type Course, type CourseContent} from '../api';
import {useT} from '../i18n';
import {btnPrimary, Modal} from '../ui';
import {categoryOf, formatBytes, itemAllowed} from '../utils';

/** Pro Abschnitt werden zunächst nur so viele Dateien gerendert (große Kurse bleiben flüssig). */
const PAGE_SIZE = 100;

export function ContentDialog(props: {
  course: Course;
  content?: CourseContent;
  excluded: string[];
  types: string[];
  onLoaded: (c: CourseContent) => void;
  onExclude: (list: string[]) => void;
  onError: (e: unknown) => void;
  onClose: () => void;
}) {
  const t = useT();
  const {course, content, onLoaded, onError} = props;
  const [err, setErr] = useState('');
  const [shown, setShown] = useState<Record<number, number>>({});

  useEffect(() => {
    if (content) return;
    let alive = true;
    api
      .scanCourse(course.id)
      .then((c) => alive && onLoaded(c))
      .catch((e) => {
        if (!alive) return;
        if (errorMessage(e) === SESSION_EXPIRED) {
          onError(e);
          props.onClose();
        } else setErr(errorMessage(e));
      });
    return () => {
      alive = false;
    };
  }, [course.id, content]);

  const skip = new Set(props.excluded);
  const setSkip = (urls: string[], on: boolean) => {
    const n = new Set(skip);
    urls.forEach((u) => (on ? n.delete(u) : n.add(u)));
    props.onExclude([...n]);
  };

  const total = content?.sections.reduce((a, s) => a + s.items.length, 0) ?? 0;
  const linkCount = content?.sections.reduce((a, s) => a + s.links.length, 0) ?? 0;
  const selected = content
    ? total - props.excluded.filter((u) => content.sections.some((s) => s.items.some((i) => i.url === u))).length
    : 0;

  return (
    <Modal title={course.name} onClose={props.onClose} wide>
      <div className="min-h-[200px] flex-1 overflow-y-auto p-5">
        {err ? (
          <p className="text-sm text-red-600">{err}</p>
        ) : !content ? (
          <div className="flex h-40 flex-col items-center justify-center gap-3 text-slate-500">
            <Loader2 className="h-7 w-7 animate-spin" /> {t('content.analyzing')}
          </div>
        ) : total === 0 && linkCount === 0 ? (
          <p className="text-sm text-slate-500">{t('content.empty')}</p>
        ) : (
          <div className="space-y-5">
            {content.sections.map((s, si) => {
              const urls = s.items.map((i) => i.url);
              const allOn = urls.every((u) => !skip.has(u));
              const limit = shown[si] ?? PAGE_SIZE;
              return (
                <div key={si}>
                  <label className="flex cursor-pointer items-center gap-2 text-sm font-semibold">
                    {s.items.length > 0 && (
                      <input
                        type="checkbox"
                        checked={allOn}
                        onChange={() => setSkip(urls, !allOn)}
                        className="h-4 w-4 accent-indigo-600"
                      />
                    )}
                    {s.title}
                    <span className="text-xs font-normal text-slate-500">({s.items.length})</span>
                  </label>
                  {s.items.length > 0 && (
                    <ul className="mt-1 divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-700 dark:border-slate-700">
                      {s.items.slice(0, limit).map((it) => {
                        const typeOff = !itemAllowed(it, props.types);
                        return (
                          <li key={it.url} className={`flex items-center gap-3 px-3 py-2 ${typeOff ? 'opacity-40' : ''}`}>
                            <input
                              type="checkbox"
                              checked={!skip.has(it.url)}
                              onChange={(e) => setSkip([it.url], e.target.checked)}
                              className="h-4 w-4 accent-indigo-600"
                            />
                            <div className="min-w-0 flex-1">
                              <div className="truncate text-sm">{it.filename}</div>
                              {it.folder && <div className="truncate text-xs text-slate-500">{it.folder}</div>}
                            </div>
                            <span className="shrink-0 rounded bg-slate-100 px-1.5 py-0.5 text-[10px] uppercase text-slate-600 dark:bg-slate-700 dark:text-slate-300">
                              {it.ext || categoryOf(it.ext)}
                            </span>
                            <span className="w-16 shrink-0 text-right text-xs text-slate-500">{formatBytes(it.size)}</span>
                          </li>
                        );
                      })}
                    </ul>
                  )}
                  {s.items.length > limit && (
                    <button
                      className="mt-1 text-xs text-indigo-600 hover:underline dark:text-indigo-400"
                      onClick={() => setShown((p) => ({...p, [si]: limit + PAGE_SIZE}))}
                    >
                      {t('content.showMore', {n: Math.min(PAGE_SIZE, s.items.length - limit)})}
                    </button>
                  )}
                  {s.links.length > 0 && (
                    <div className="mt-2 rounded-lg bg-slate-50 p-2 text-xs dark:bg-slate-900">
                      <div className="mb-1 font-medium text-slate-500">{t('content.links')}</div>
                      <ul className="space-y-0.5">
                        {s.links.map((l) => (
                          <li key={l.url} className="flex items-center gap-1 truncate">
                            <ExternalLink className="h-3 w-3 shrink-0 text-slate-400" />
                            <span className="truncate">{l.name || l.url}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
      <div className="flex items-center justify-between border-t border-slate-200 px-5 py-3 dark:border-slate-700">
        <span className="text-xs text-slate-500">
          {content ? t('content.selectedOf', {n: selected, total}) : ''}
        </span>
        <button className={btnPrimary} onClick={props.onClose}>
          {t('common.done')}
        </button>
      </div>
    </Modal>
  );
}
