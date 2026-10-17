import {useEffect} from 'react';
import {X} from 'lucide-react';

export const btnPrimary =
  'inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-600 px-4 py-2.5 text-sm font-semibold text-white shadow-sm transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50';
export const btnGhost =
  'inline-flex items-center justify-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700';

export function CenterCard({children}: {children: React.ReactNode}) {
  return (
    <div className="flex flex-1 items-center justify-center p-6">
      <div className="w-full max-w-md rounded-2xl border border-slate-200 bg-white p-8 text-center shadow-sm dark:border-slate-700 dark:bg-slate-800">
        {children}
      </div>
    </div>
  );
}

export function Modal(props: {title: string; onClose?: () => void; children: React.ReactNode; wide?: boolean}) {
  const {onClose} = props;
  useEffect(() => {
    if (!onClose) return;
    const h = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [onClose]);
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4">
      <div
        className={`flex max-h-full w-full flex-col rounded-2xl bg-white shadow-xl dark:bg-slate-800 ${
          props.wide ? 'max-w-3xl' : 'max-w-lg'
        }`}
      >
        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-3 dark:border-slate-700">
          <h2 className="truncate text-base font-semibold">{props.title}</h2>
          {props.onClose && (
            <button onClick={props.onClose} className="rounded p-1 hover:bg-slate-100 dark:hover:bg-slate-700">
              <X className="h-5 w-5" />
            </button>
          )}
        </div>
        {props.children}
      </div>
    </div>
  );
}

export function Toggle(props: {label: string; hint?: string; checked: boolean; onChange: (v: boolean) => void}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 py-2">
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(e) => props.onChange(e.target.checked)}
        className="mt-1 h-4 w-4 accent-indigo-600"
      />
      <span>
        <span className="text-sm font-medium">{props.label}</span>
        {props.hint && <span className="block text-xs text-slate-500 dark:text-slate-400">{props.hint}</span>}
      </span>
    </label>
  );
}

export function Bar({value, small}: {value: number; small?: boolean}) {
  return (
    <div className={`overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700 ${small ? 'h-1.5' : 'h-2.5'}`}>
      <div className="h-full bg-indigo-600 transition-all" style={{width: `${Math.max(0, Math.min(100, value))}%`}} />
    </div>
  );
}
