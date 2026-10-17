import {ChevronRight, ShieldCheck} from 'lucide-react';
import {useT} from '../i18n';
import {Logo} from '../Logo';
import {btnPrimary, CenterCard} from '../ui';

export function ConnectScreen(props: {
  url: string;
  recent: string[];
  error: string;
  remember: boolean;
  restoring: boolean;
  onUrl: (u: string) => void;
  onRemember: (v: boolean) => void;
  onStart: () => void;
}) {
  const t = useT();
  if (props.restoring) {
    return (
      <CenterCard>
        <div className="mx-auto w-fit">
          <Logo size={72} />
        </div>
        <p className="mt-6 text-sm text-slate-500 dark:text-slate-400">{t('connect.restoring')}</p>
      </CenterCard>
    );
  }
  return (
    <CenterCard>
      <div className="mx-auto w-fit">
        <Logo size={72} />
      </div>
      <h1 className="mt-5 text-2xl font-bold">Moodle Scraper</h1>
      <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">
        {t('connect.subtitle')}
      </p>
      <form
        className="mt-6 text-left"
        onSubmit={(e) => {
          e.preventDefault();
          props.onStart();
        }}
      >
        <label className="text-xs font-medium text-slate-600 dark:text-slate-300">{t('connect.urlLabel')}</label>
        <input
          autoFocus
          value={props.url}
          onChange={(e) => props.onUrl(e.target.value)}
          placeholder={t('connect.urlPlaceholder')}
          className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2.5 text-sm outline-none focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-600 dark:bg-slate-900"
        />
        {props.recent.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1.5">
            {props.recent.map((u) => (
              <button
                type="button"
                key={u}
                onClick={() => props.onUrl(u)}
                className="rounded-full bg-slate-100 px-2.5 py-1 text-xs text-slate-600 hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300"
              >
                {u.replace(/^https?:\/\//, '').replace(/\/$/, '')}
              </button>
            ))}
          </div>
        )}
        <label className="mt-4 flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={props.remember}
            onChange={(e) => props.onRemember(e.target.checked)}
            className="mt-0.5 h-4 w-4 accent-indigo-600"
          />
          <span>
            {t('connect.remember')}
            <span className="block text-xs text-slate-500 dark:text-slate-400">{t('connect.rememberHint')}</span>
          </span>
        </label>
        {props.error && <p className="mt-3 text-sm text-red-600">{props.error}</p>}
        <button type="submit" className={`${btnPrimary} mt-5 w-full`} disabled={!props.url.trim()}>
          {t('connect.submit')} <ChevronRight className="h-4 w-4" />
        </button>
      </form>
      <p className="mt-5 flex items-start gap-2 text-left text-xs text-slate-500 dark:text-slate-400">
        <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
        {t('connect.privacy')}
      </p>
    </CenterCard>
  );
}
