import {Eye, Star} from 'lucide-react';
import type {Course} from '../api';
import {btnGhost} from '../ui';
import {useT} from '../i18n';
import {hueOf} from '../utils';

export function CourseCard(props: {
  course: Course;
  checked: boolean;
  excludedCount: number;
  onToggle: () => void;
  onOpen: () => void;
}) {
  const t = useT();
  const k = props.course;
  const hue = hueOf(k.name);
  return (
    <div
      onClick={props.onToggle}
      className={`flex cursor-pointer items-center gap-3 rounded-xl border p-3 transition ${
        props.checked
          ? 'border-indigo-500 bg-indigo-50 dark:bg-indigo-950/40'
          : 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-800 dark:hover:border-slate-600'
      }`}
    >
      <input
        type="checkbox"
        checked={props.checked}
        onChange={props.onToggle}
        onClick={(e) => e.stopPropagation()}
        className="h-4 w-4 shrink-0 accent-indigo-600"
      />
      <div
        className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg text-sm font-bold text-white"
        style={{background: `hsl(${hue} 60% 48%)`}}
      >
        {k.name.trim().slice(0, 2).toUpperCase()}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate text-sm font-semibold">{k.name}</span>
          {k.favourite && <Star className="h-3.5 w-3.5 shrink-0 fill-amber-400 text-amber-400" />}
        </div>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-slate-500 dark:text-slate-400">
          {k.category && <span className="truncate">{k.category}</span>}
          {k.ended && <span className="rounded bg-slate-200 px-1.5 dark:bg-slate-700">{t('card.ended')}</span>}
          {k.hidden && <span className="rounded bg-slate-200 px-1.5 dark:bg-slate-700">{t('card.hidden')}</span>}
          {props.excludedCount > 0 && <span>{t('card.excluded', {n: props.excludedCount})}</span>}
        </div>
        {k.progress != null && (
          <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700">
            <div className="h-full bg-emerald-500" style={{width: `${Math.round(k.progress)}%`}} />
          </div>
        )}
      </div>
      <button
        className={btnGhost}
        title={t('card.filesTitle')}
        onClick={(e) => {
          e.stopPropagation();
          props.onOpen();
        }}
      >
        <Eye className="h-4 w-4" /> <span className="hidden sm:inline">{t('card.files')}</span>
      </button>
    </div>
  );
}
