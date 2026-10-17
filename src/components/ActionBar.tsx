import {Download, FolderOpen} from 'lucide-react';
import {useT} from '../i18n';
import {btnGhost, btnPrimary} from '../ui';
import type {Settings} from '../utils';

interface Props {
  selectedCount: number;
  settings: Settings;
  busy: boolean;
  onOpenSettings: () => void;
  onClear: () => void;
  onDownload: () => void;
}

export function ActionBar({selectedCount, settings, busy, onOpenSettings, onClear, onDownload}: Props) {
  const t = useT();
  return (
    <footer className="flex flex-wrap items-center gap-4 border-t border-slate-200 bg-white px-6 py-3 dark:border-slate-700 dark:bg-slate-800">
      <div className="min-w-0 flex-1">
        <div className="text-sm font-semibold">
          {selectedCount === 0
            ? t('action.none')
            : selectedCount === 1
              ? t('action.selectedOne')
              : t('action.selectedMany', {n: selectedCount})}
        </div>
        <button
          onClick={onOpenSettings}
          className="mt-0.5 flex max-w-full items-center gap-1 truncate text-xs text-slate-500 hover:text-indigo-600 dark:text-slate-400"
          title={t('action.settingsTitle')}
        >
          <FolderOpen className="h-3.5 w-3.5 shrink-0" />
          <span className="truncate">
            {settings.destDir || t('action.noDest')} · {settings.zip ? t('action.zip') : t('action.folder')} ·{' '}
            {settings.incremental ? t('action.onlyNew') : t('action.everything')} ·{' '}
            {t('action.types', {n: settings.types.length})}
          </span>
        </button>
      </div>
      {selectedCount > 0 && (
        <button className={btnGhost} onClick={onClear}>
          {t('action.clear')}
        </button>
      )}
      <button className={btnPrimary} disabled={!selectedCount || busy} onClick={onDownload}>
        <Download className="h-4 w-4" /> {selectedCount > 0 ? t('action.downloadN', {n: selectedCount}) : t('action.download')}
      </button>
    </footer>
  );
}
