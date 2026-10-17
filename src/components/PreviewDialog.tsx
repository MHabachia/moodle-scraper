import type {CoursePreview} from '../api';
import {useT} from '../i18n';
import {btnGhost, btnPrimary, Modal} from '../ui';
import {formatBytes} from '../utils';

/** Gesamtzahlen einer Vorschau. */
export function previewTotals(list: CoursePreview[]) {
  const files = list.reduce((a, p) => a + p.newFiles + p.changedFiles, 0);
  const bytes = list.reduce((a, p) => a + p.bytes, 0);
  const links = list.reduce((a, p) => a + p.links, 0);
  return {files, bytes, links, nothing: files === 0 && links === 0};
}

export function PreviewDialog(props: {
  previews: CoursePreview[];
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const t = useT();
  const total = previewTotals(props.previews);
  return (
    <Modal title={t('preview.title')} onClose={props.onCancel} wide>
      <div className="flex-1 space-y-4 overflow-y-auto p-5">
        <p className="text-sm text-slate-600 dark:text-slate-300">
          {total.nothing ? t('preview.nothing') : t('preview.intro')}
        </p>
        <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-700 dark:border-slate-700">
          {props.previews.map((p) => (
            <li key={p.courseId} className="px-3 py-2">
              <div className="truncate text-sm font-medium">{p.name}</div>
              <div className="mt-0.5 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-slate-500">
                <span className="text-emerald-600 dark:text-emerald-400">
                  {p.newFiles} {t('preview.new')}
                </span>
                <span className="text-amber-600 dark:text-amber-400">
                  {p.changedFiles} {t('preview.changed')}
                </span>
                <span>
                  {p.unchanged} {t('preview.unchanged')}
                </span>
                {p.blocked > 0 && (
                  <span>
                    {p.blocked} {t('preview.blocked')}
                  </span>
                )}
                {p.tooLarge > 0 && (
                  <span>
                    {p.tooLarge} {t('preview.tooLarge')}
                  </span>
                )}
                {p.links > 0 && <span>{t('preview.links', {n: p.links})}</span>}
                {p.bytes > 0 && <span>{formatBytes(p.bytes)}</span>}
                {p.unknownSize > 0 && <span>{t('preview.unknownSize', {n: p.unknownSize})}</span>}
              </div>
            </li>
          ))}
        </ul>
        {!total.nothing && (
          <p className="text-sm font-medium">{t('preview.total', {files: total.files, size: formatBytes(total.bytes)})}</p>
        )}
      </div>
      <div className="flex justify-end gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-700">
        <button className={btnGhost} onClick={props.onCancel}>
          {t('common.cancel')}
        </button>
        <button className={btnPrimary} onClick={props.onConfirm} disabled={total.nothing}>
          {t('preview.start')}
        </button>
      </div>
    </Modal>
  );
}
