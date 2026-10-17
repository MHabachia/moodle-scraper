import {useState} from 'react';
import {FolderOpen} from 'lucide-react';
import {api, errorMessage} from '../api';
import {useT} from '../i18n';
import {btnGhost, btnPrimary, Modal, Toggle} from '../ui';
import {CATEGORIES, DIR_PRESETS, type Settings} from '../utils';

const SYNC_HOURS = [1, 3, 6, 12, 24];

const label = 'text-xs font-medium text-slate-600 dark:text-slate-300';
const field = 'rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm dark:border-slate-600 dark:bg-slate-900';

function Section({title, children}: {title: string; children: React.ReactNode}) {
  return (
    <section className="space-y-4">
      <h3 className="border-b border-slate-200 pb-1 text-xs font-semibold uppercase tracking-wide text-slate-500 dark:border-slate-700 dark:text-slate-400">
        {title}
      </h3>
      {children}
    </section>
  );
}

function Advanced({title, children}: {title: string; children: React.ReactNode}) {
  return (
    <details className="group rounded-lg border border-slate-200 dark:border-slate-700">
      <summary className="flex select-none items-center justify-between px-3 py-2 text-sm font-medium">
        {title}
        <span className="text-slate-400 transition group-open:rotate-90">›</span>
      </summary>
      <div className="space-y-4 border-t border-slate-200 p-3 dark:border-slate-700">{children}</div>
    </details>
  );
}

export function SettingsDialog(props: {
  settings: Settings;
  onChange: (p: Partial<Settings>) => void;
  onClose: () => void;
  /** Benachrichtigungen erlauben (beim Aktivieren von Auto-Sync). */
  onEnableAutoSync: () => void;
}) {
  const t = useT();
  const s = props.settings;
  const [note, setNote] = useState('');

  const toggleType = (id: string) =>
    props.onChange({types: s.types.includes(id) ? s.types.filter((x) => x !== id) : [...s.types, id]});

  const run = async (fn: () => Promise<string | null>) => {
    try {
      setNote((await fn()) ?? '');
    } catch (e) {
      setNote(errorMessage(e));
    }
  };

  return (
    <Modal title={t('settings.title')} onClose={props.onClose}>
      <div className="space-y-6 overflow-y-auto p-5">
        <p className="text-xs text-slate-500 dark:text-slate-400">{t('settings.perSite')}</p>

        <Section title={t('settings.download')}>
          <div className="flex gap-2">
            <input readOnly value={s.destDir} className={`${field} min-w-0 flex-1 bg-slate-50 dark:bg-slate-900`} />
            <button
              className={btnGhost}
              onClick={async () => {
                const d = await api.pickFolder(s.destDir || undefined);
                if (d) props.onChange({destDir: d});
              }}
            >
              <FolderOpen className="h-4 w-4" /> {t('common.choose')}
            </button>
          </div>

          <div>
            <div className={label}>{t('settings.format')}</div>
            <div className="mt-1 grid grid-cols-2 gap-2">
              {[
                {zip: false, title: t('settings.formatFolder'), hint: t('settings.formatFolderHint')},
                {zip: true, title: t('settings.formatZip'), hint: t('settings.formatZipHint')},
              ].map((o) => (
                <button
                  key={String(o.zip)}
                  onClick={() => props.onChange({zip: o.zip})}
                  className={`rounded-lg border p-3 text-left transition ${
                    s.zip === o.zip
                      ? 'border-indigo-500 bg-indigo-50 dark:bg-indigo-950/40'
                      : 'border-slate-300 dark:border-slate-600'
                  }`}
                >
                  <div className="text-sm font-semibold">{o.title}</div>
                  <div className="text-xs text-slate-500 dark:text-slate-400">{o.hint}</div>
                </button>
              ))}
            </div>
          </div>

          <div>
            <div className={label}>{t('settings.types')}</div>
            <div className="mt-2 flex flex-wrap gap-2">
              {CATEGORIES.map((c) => (
                <button
                  key={c.id}
                  onClick={() => toggleType(c.id)}
                  className={`rounded-full border px-3 py-1 text-xs font-medium transition ${
                    s.types.includes(c.id)
                      ? 'border-indigo-500 bg-indigo-600 text-white'
                      : 'border-slate-300 text-slate-600 dark:border-slate-600 dark:text-slate-300'
                  }`}
                >
                  {t(`cat.${c.id}` as 'cat.docs')}
                </button>
              ))}
            </div>
          </div>

          {s.types.includes('programs') && (
            <p className="rounded-lg border border-amber-300 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-200">
              {t('settings.programsWarning')}
            </p>
          )}

          <div className="divide-y divide-slate-200 dark:divide-slate-700">
            <Toggle
              label={t('settings.incremental')}
              hint={t('settings.incrementalHint')}
              checked={s.incremental}
              onChange={(v) => props.onChange({incremental: v})}
            />
            <Toggle
              label={t('settings.preview')}
              hint={t('settings.previewHint')}
              checked={s.preview}
              onChange={(v) => props.onChange({preview: v})}
            />
          </div>
        </Section>

        <Advanced title={t('settings.advanced')}>
          <div>
            <div className={label}>{t('settings.structure')}</div>
            <div className="mt-1 flex flex-wrap gap-2">
              {DIR_PRESETS.map((p) => (
                <button
                  key={p.id}
                  onClick={() => props.onChange({dirTemplate: p.tpl})}
                  className={`rounded-full border px-3 py-1 text-xs font-medium transition ${
                    s.dirTemplate === p.tpl
                      ? 'border-indigo-500 bg-indigo-600 text-white'
                      : 'border-slate-300 text-slate-600 dark:border-slate-600 dark:text-slate-300'
                  }`}
                >
                  {t(`preset.${p.id}` as 'preset.sections')}
                </button>
              ))}
            </div>
            <input
              value={s.dirTemplate}
              onChange={(e) => props.onChange({dirTemplate: e.target.value})}
              placeholder={t('settings.templatePlaceholder')}
              className={`${field} mt-2 w-full font-mono`}
            />
            <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">{t('settings.templateHelp')}</p>
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <div>
              <div className={label}>{t('settings.naming')}</div>
              <select
                value={s.courseNaming}
                onChange={(e) => props.onChange({courseNaming: e.target.value as Settings['courseNaming']})}
                className={`${field} mt-1 w-full`}
              >
                <option value="full">{t('naming.full')}</option>
                <option value="clean">{t('naming.clean')}</option>
                <option value="short">{t('naming.short')}</option>
              </select>
            </div>
            <div>
              <div className={label}>{t('settings.maxSize')}</div>
              <select
                value={s.maxSizeMb}
                onChange={(e) => props.onChange({maxSizeMb: Number(e.target.value)})}
                className={`${field} mt-1 w-full`}
              >
                <option value={0}>{t('settings.unlimited')}</option>
                <option value={25}>25 MB</option>
                <option value={100}>100 MB</option>
                <option value={500}>500 MB</option>
              </select>
            </div>
            <div>
              <div className={label}>{t('settings.parallel')}</div>
              <select
                value={s.parallel}
                onChange={(e) => props.onChange({parallel: Number(e.target.value)})}
                className={`${field} mt-1 w-full`}
              >
                {[1, 2, 3, 4, 5, 6].map((n) => (
                  <option key={n} value={n}>
                    {n}
                    {n === 3 ? ` ${t('settings.recommended')}` : ''}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="divide-y divide-slate-200 dark:divide-slate-700">
            <Toggle
              label={t('settings.saveLinks')}
              hint={t('settings.saveLinksHint')}
              checked={s.saveLinks}
              onChange={(v) => props.onChange({saveLinks: v})}
            />
            <Toggle
              label={t('settings.autoSync')}
              hint={t('settings.autoSyncHint')}
              checked={s.autoSync}
              onChange={(v) => {
                props.onChange({autoSync: v});
                if (v) props.onEnableAutoSync();
              }}
            />
          </div>
          {s.autoSync && (
            <div className="flex items-center gap-3">
              <span className={label}>{t('settings.interval')}</span>
              <select
                value={s.autoSyncHours}
                onChange={(e) => props.onChange({autoSyncHours: Number(e.target.value)})}
                className={field}
              >
                {SYNC_HOURS.map((h) => (
                  <option key={h} value={h}>
                    {t('settings.hours', {n: h})}
                  </option>
                ))}
              </select>
            </div>
          )}
        </Advanced>

        <Section title={t('settings.general')}>
          <div>
            <div className={label}>{t('settings.language')}</div>
            <select
              value={s.language}
              onChange={(e) => props.onChange({language: e.target.value as Settings['language']})}
              className={`${field} mt-1 w-full sm:w-64`}
            >
              <option value="auto">{t('lang.auto')}</option>
              <option value="de">{t('lang.de')}</option>
              <option value="en">{t('lang.en')}</option>
            </select>
          </div>
          <div className="divide-y divide-slate-200 dark:divide-slate-700">
            <Toggle
              label={t('settings.remember')}
              hint={t('settings.rememberHint')}
              checked={s.rememberLogin}
              onChange={(v) => props.onChange({rememberLogin: v})}
            />
            <Toggle
              label={t('settings.background')}
              hint={t('settings.backgroundHint')}
              checked={s.background}
              onChange={(v) => props.onChange({background: v})}
            />
          </div>
        </Section>

        <Section title={t('settings.tools')}>
          <div className="flex flex-wrap gap-2">
            <button
              className={btnGhost}
              onClick={() =>
                run(async () => {
                  await api.clearScanCache();
                  return t('settings.cacheCleared');
                })
              }
            >
              {t('settings.clearCache')}
            </button>
            <button
              className={btnGhost}
              onClick={() =>
                run(async () => {
                  const path = await api.pickSavePath('moodle-scraper-diagnose.txt', 'txt');
                  if (!path) return null;
                  await api.exportDiagnostics(path);
                  return t('settings.diagnosticsSaved');
                })
              }
            >
              {t('settings.diagnostics')}
            </button>
          </div>
          {note && <p className="text-xs text-slate-500 dark:text-slate-400">{note}</p>}
        </Section>
      </div>
      <div className="flex justify-end border-t border-slate-200 px-5 py-3 dark:border-slate-700">
        <button className={btnPrimary} onClick={props.onClose}>
          {t('common.done')}
        </button>
      </div>
    </Modal>
  );
}
