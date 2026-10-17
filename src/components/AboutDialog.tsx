import {useEffect, useState} from 'react';
import {ExternalLink} from 'lucide-react';
import {api} from '../api';
import {Logo} from '../Logo';
import {useT} from '../i18n';
import {btnGhost, btnPrimary, Modal} from '../ui';

const REPO_URL = 'https://github.com/MHabachia/moodle-scraper';

export function AboutDialog(props: {onClose: () => void}) {
  const t = useT();
  const [version, setVersion] = useState('');

  useEffect(() => {
    // Außerhalb von Tauri (Tests, Browser) gibt es keine Version; dann bleibt die Zeile leer
    api.appVersion().then(setVersion).catch(() => undefined);
  }, []);

  const stack = [t('about.stackCore'), t('about.stackUi'), t('about.stackStore'), t('about.stackTests')];

  return (
    <Modal title={t('about.title')} onClose={props.onClose}>
      <div className="space-y-4 overflow-y-auto px-5 py-4 text-sm">
        <div className="flex items-center gap-3">
          <Logo size={44} />
          <div>
            <div className="font-bold">Moodle Scraper</div>
            {version && <div className="text-xs text-slate-500 dark:text-slate-400">{t('about.version', {version})}</div>}
          </div>
        </div>
        <p>{t('about.purpose')}</p>
        <p className="text-slate-600 dark:text-slate-300">{t('about.privacy')}</p>
        <div>
          <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t('about.stack')}
          </h3>
          <ul className="list-inside list-disc space-y-0.5">
            {stack.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ul>
        </div>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          {t('about.license')} {t('about.notice')}
        </p>
      </div>
      <div className="flex items-center justify-between gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-700">
        <button className={btnGhost} onClick={() => void api.openUrl(REPO_URL).catch(() => undefined)}>
          <ExternalLink className="h-4 w-4" /> {t('about.repo')}
        </button>
        <button className={btnPrimary} onClick={props.onClose}>
          {t('common.close')}
        </button>
      </div>
    </Modal>
  );
}
