import {Search} from 'lucide-react';
import {Rich, useT} from '../i18n';
import {FILTERS} from '../lib/courses';
import type {Filter, Sort} from '../types';

interface Props {
  search: string;
  onSearch: (v: string) => void;
  sort: Sort;
  onSort: (v: Sort) => void;
  filter: Filter;
  onFilter: (v: Filter) => void;
  counts: Record<Filter, number>;
  allSelected: boolean;
  onToggleAll: () => void;
  onSelectActive: () => void;
  /** Kurzanleitung zeigen (solange noch nichts gewählt ist) */
  showHint: boolean;
}

export function CourseToolbar(p: Props) {
  const t = useT();
  return (
    <div className="space-y-3 border-b border-slate-200 bg-white px-6 py-3 dark:border-slate-700 dark:bg-slate-800">
      {p.showHint && (
        <p className="text-sm text-slate-600 dark:text-slate-300">
          <Rich text={t('toolbar.howto')} />
        </p>
      )}
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative min-w-[220px] flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" />
          <input
            value={p.search}
            onChange={(e) => p.onSearch(e.target.value)}
            placeholder={t('toolbar.search')}
            className="w-full rounded-lg border border-slate-300 bg-white py-2 pl-9 pr-3 text-sm outline-none focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-600 dark:bg-slate-900"
          />
        </div>
        <select
          value={p.sort}
          onChange={(e) => p.onSort(e.target.value as Sort)}
          className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm dark:border-slate-600 dark:bg-slate-900"
        >
          <option value="name">{t('toolbar.sortName')}</option>
          <option value="progress">{t('toolbar.sortProgress')}</option>
        </select>
        <label className="flex cursor-pointer items-center gap-2 text-sm">
          <input type="checkbox" checked={p.allSelected} onChange={p.onToggleAll} className="h-4 w-4 accent-indigo-600" />
          {t('toolbar.selectAll')}
        </label>
        <button className="text-sm text-indigo-600 hover:underline dark:text-indigo-400" onClick={p.onSelectActive}>
          {t('toolbar.selectActive')}
        </button>
      </div>
      <div className="flex flex-wrap gap-2">
        {FILTERS.map((f) => (
          <button
            key={f}
            onClick={() => p.onFilter(f)}
            className={`rounded-full px-3 py-1 text-xs font-medium transition ${
              p.filter === f
                ? 'bg-indigo-600 text-white'
                : 'bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300 dark:hover:bg-slate-600'
            }`}
          >
            {t(`filter.${f}`)} <span className="opacity-70">{p.counts[f]}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
