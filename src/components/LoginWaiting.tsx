import {Loader2} from 'lucide-react';
import {useT} from '../i18n';
import {btnGhost, CenterCard} from '../ui';

interface Props {
  error: string;
  onReopen: () => void;
  onCancel: () => void;
}

export function LoginWaiting({error, onReopen, onCancel}: Props) {
  const t = useT();
  return (
    <CenterCard>
      <Loader2 className="mx-auto h-10 w-10 animate-spin text-indigo-500" />
      <h2 className="mt-5 text-xl font-semibold">{t('login.title')}</h2>
      <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">
        {t('login.text')}
      </p>
      <div className="mt-6 flex justify-center gap-3">
        <button className={btnGhost} onClick={onReopen}>
          {t('login.reopen')}
        </button>
        <button className={btnGhost} onClick={onCancel}>
          {t('common.cancel')}
        </button>
      </div>
      {error && <p className="mt-4 text-sm text-red-600">{error}</p>}
    </CenterCard>
  );
}
