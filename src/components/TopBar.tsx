import {CalendarClock, ChevronRight, Info, LogOut, Monitor, Moon, RefreshCw, Settings as SettingsIcon, Sun} from 'lucide-react';
import type {SessionInfo} from '../api';
import {useT, type Key} from '../i18n';
import {Logo} from '../Logo';
import {btnGhost} from '../ui';
import type {Settings} from '../utils';

const STEPS: Key[] = ['step.connect', 'step.login', 'step.courses', 'step.download'];

export function TopBar(props: {
  step: number;
  theme: Settings['theme'];
  onTheme: (t: Settings['theme']) => void;
  session: SessionInfo | null;
  loading: boolean;
  onRefresh: () => void;
  onSettings: () => void;
  onDeadlines: () => void;
  onLogout: () => void;
  onAbout: () => void;
}) {
  const t = useT();
  const next = {system: 'light', light: 'dark', dark: 'system'} as const;
  const ThemeIcon = props.theme === 'dark' ? Moon : props.theme === 'light' ? Sun : Monitor;
  const themeLabel = t(`theme.${props.theme}`);
  return (
    <header className="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-2.5 dark:border-slate-700 dark:bg-slate-800">
      <div className="flex items-center gap-2.5">
        <Logo size={30} />
        <div className="leading-tight">
          <div className="text-sm font-bold">Moodle Scraper</div>
          {props.session && (
            <div className="max-w-[200px] truncate text-[11px] text-slate-500 dark:text-slate-400">
              {props.session.user ? `${props.session.user} · ` : ''}
              {props.session.host}
            </div>
          )}
        </div>
      </div>

      <ol className="mx-auto hidden items-center gap-1 lg:flex">
        {STEPS.map((key, i) => {
          const n = i + 1;
          const done = n < props.step;
          const active = n === props.step;
          return (
            <li key={key} className="flex items-center gap-1 whitespace-nowrap">
              <span
                className={`flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-bold ${
                  active
                    ? 'bg-indigo-600 text-white'
                    : done
                      ? 'bg-emerald-500 text-white'
                      : 'bg-slate-200 text-slate-500 dark:bg-slate-700 dark:text-slate-400'
                }`}
              >
                {done ? '✓' : n}
              </span>
              <span
                className={`text-xs ${active ? 'font-semibold' : 'text-slate-500 dark:text-slate-400'}`}
              >
                {t(key)}
              </span>
              {n < STEPS.length && <ChevronRight className="h-3.5 w-3.5 text-slate-300 dark:text-slate-600" />}
            </li>
          );
        })}
      </ol>

      <div className="mx-auto text-xs font-medium text-slate-500 lg:hidden dark:text-slate-400">
        {props.step}/{STEPS.length} · {t(STEPS[props.step - 1])}
      </div>

      <div className="flex items-center gap-2">
        {props.session && (
          <>
            <button className={btnGhost} onClick={props.onRefresh} disabled={props.loading} title={t('topbar.refresh')} aria-label={t('topbar.refresh')}>
              <RefreshCw className={`h-4 w-4 ${props.loading ? 'animate-spin' : ''}`} />
            </button>
            <button className={btnGhost} onClick={props.onDeadlines} title={t('topbar.deadlines')}>
              <CalendarClock className="h-4 w-4" /> <span className="hidden xl:inline">{t('topbar.deadlines')}</span>
            </button>
            <button className={btnGhost} onClick={props.onSettings} title={t('topbar.settingsTitle')}>
              <SettingsIcon className="h-4 w-4" /> <span className="hidden xl:inline">{t('topbar.settings')}</span>
            </button>
            <button className={btnGhost} onClick={props.onLogout} title={t('topbar.logout')} aria-label={t('topbar.logout')}>
              <LogOut className="h-4 w-4" />
            </button>
          </>
        )}
        <button className={btnGhost} onClick={props.onAbout} title={t('topbar.about')} aria-label={t('topbar.about')}>
          <Info className="h-4 w-4" />
        </button>
        <button
          className={btnGhost}
          onClick={() => props.onTheme(next[props.theme])}
          title={t('topbar.theme', {theme: themeLabel})}
        >
          <ThemeIcon className="h-4 w-4" /> <span className="hidden xl:inline">{themeLabel}</span>
        </button>
      </div>
    </header>
  );
}
