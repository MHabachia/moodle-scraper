import {useState} from 'react';
import {Loader2} from 'lucide-react';
import type {UpdateState} from '../hooks/useUpdater';
import {useT} from '../i18n';
import {btnGhost, btnPrimary} from '../ui';

/** Schmale Leiste unter der oberen Leiste, wenn eine neue Version bereitsteht. */
export function UpdateBanner(props: {state: UpdateState; busy: boolean; onInstall: () => void}) {
  const t = useT();
  const [dismissed, setDismissed] = useState(false);
  const {state} = props;
  if (state.status === 'installing') {
    return (
      <div className="flex items-center gap-2 border-b border-indigo-200 bg-indigo-50 px-5 py-2 text-sm dark:border-indigo-900 dark:bg-indigo-950">
        <Loader2 className="h-4 w-4 animate-spin" /> {t('update.installing')}
      </div>
    );
  }
  if (state.status !== 'available' || dismissed) return null;
  return (
    <div className="flex items-center gap-3 border-b border-indigo-200 bg-indigo-50 px-5 py-2 text-sm dark:border-indigo-900 dark:bg-indigo-950">
      <span className="font-medium">{t('update.available', {version: state.version})}</span>
      {props.busy && <span className="text-xs text-slate-500 dark:text-slate-400">{t('update.busy')}</span>}
      <div className="ml-auto flex gap-2">
        <button className={btnGhost} onClick={() => setDismissed(true)}>
          {t('update.later')}
        </button>
        <button className={btnPrimary} onClick={props.onInstall} disabled={props.busy}>
          {t('update.install')}
        </button>
      </div>
    </div>
  );
}
