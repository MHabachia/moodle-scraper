import {AlertTriangle, Loader2} from 'lucide-react';
import type {Course} from '../api';
import {useT} from '../i18n';
import {CourseCard} from './CourseCard';

interface Props {
  courses: Course[];
  visible: Course[];
  loading: boolean;
  error: string;
  selected: Set<string>;
  excluded: Record<string, string[]>;
  onToggle: (id: string) => void;
  onOpen: (c: Course) => void;
}

export function CourseList(p: Props) {
  const t = useT();
  return (
    <main className="flex-1 overflow-y-auto px-6 py-4">
      {p.error && (
        <div className="mb-4 flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" /> {p.error}
        </div>
      )}
      {p.loading && !p.courses.length ? (
        <div className="flex h-64 flex-col items-center justify-center gap-3 text-slate-500">
          <Loader2 className="h-8 w-8 animate-spin" /> {t('list.loading')}
        </div>
      ) : p.visible.length === 0 ? (
        <div className="flex h-64 items-center justify-center text-slate-500">
          {p.courses.length ? t('list.noMatch') : t('list.none')}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
          {p.visible.map((k) => (
            <CourseCard
              key={k.id}
              course={k}
              checked={p.selected.has(k.id)}
              excludedCount={p.excluded[k.id]?.length ?? 0}
              onToggle={() => p.onToggle(k.id)}
              onOpen={() => p.onOpen(k)}
            />
          ))}
        </div>
      )}
    </main>
  );
}
