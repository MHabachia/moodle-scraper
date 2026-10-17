import {useEffect, useState} from 'react';
import {Loader2} from 'lucide-react';
import {api, errorMessage, SESSION_EXPIRED, type Deadline} from '../api';
import {useLanguage, useT, type Translator} from '../i18n';
import {btnGhost, btnPrimary, Modal} from '../ui';

const DAY = 86_400;

/** "heute", "morgen", "in 3 Tagen" oder "überfällig". */
export function dueLabel(t: Translator, due: number, now: number): string {
  const days = Math.floor((due - now) / DAY);
  if (due < now) return t('deadlines.overdue');
  if (days <= 0) return t('deadlines.today');
  if (days === 1) return t('deadlines.tomorrow');
  return t('deadlines.inDays', {n: days});
}

export function DeadlinesDialog(props: {onError: (e: unknown) => void; onClose: () => void}) {
  const t = useT();
  const lang = useLanguage();
  const [items, setItems] = useState<Deadline[] | null>(null);
  const [note, setNote] = useState('');
  const {onError, onClose} = props;

  useEffect(() => {
    let alive = true;
    api
      .listDeadlines()
      .then((d) => alive && setItems(d))
      .catch((e) => {
        if (!alive) return;
        if (errorMessage(e) === SESSION_EXPIRED) {
          onError(e);
          onClose();
        } else {
          setNote(errorMessage(e));
          setItems([]);
        }
      });
    return () => {
      alive = false;
    };
  }, [onError, onClose]);

  const exportIcs = async () => {
    if (!items?.length) return;
    try {
      const path = await api.pickSavePath('moodle-fristen.ics', 'ics');
      if (!path) return;
      await api.exportIcs(path, items);
      setNote(t('deadlines.exported'));
    } catch (e) {
      setNote(errorMessage(e));
    }
  };

  const now = Math.floor(Date.now() / 1000);
  const fmt = new Intl.DateTimeFormat(lang, {dateStyle: 'medium', timeStyle: 'short'});

  return (
    <Modal title={t('deadlines.title')} onClose={onClose} wide>
      <div className="min-h-[160px] flex-1 overflow-y-auto p-5">
        {items === null ? (
          <div className="flex h-32 flex-col items-center justify-center gap-3 text-slate-500">
            <Loader2 className="h-7 w-7 animate-spin" /> {t('deadlines.loading')}
          </div>
        ) : items.length === 0 ? (
          <p className="text-sm text-slate-500">{note || t('deadlines.empty')}</p>
        ) : (
          <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-700 dark:border-slate-700">
            {items.map((d) => (
              <li key={`${d.id}-${d.due}`} className="flex items-center gap-3 px-3 py-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{d.name}</div>
                  <div className="truncate text-xs text-slate-500">{d.course}</div>
                </div>
                <div className="shrink-0 text-right text-xs">
                  <div>{fmt.format(new Date(d.due * 1000))}</div>
                  <div className={d.due < now ? 'text-red-600' : 'text-slate-500'}>{dueLabel(t, d.due, now)}</div>
                </div>
              </li>
            ))}
          </ul>
        )}
        {note && items && items.length > 0 && <p className="mt-3 text-xs text-slate-500">{note}</p>}
      </div>
      <div className="flex justify-end gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-700">
        <button className={btnGhost} onClick={exportIcs} disabled={!items?.length}>
          {t('deadlines.export')}
        </button>
        <button className={btnPrimary} onClick={onClose}>
          {t('common.close')}
        </button>
      </div>
    </Modal>
  );
}
