import {Component, type ErrorInfo, type ReactNode} from 'react';
import {translate, resolveLanguage} from '../i18n';
import {btnPrimary} from '../ui';

interface State {
  error: Error | null;
}

/** Fängt unerwartete Render-Fehler ab, statt ein leeres Fenster zu zeigen. */
export class ErrorBoundary extends Component<{children: ReactNode}, State> {
  state: State = {error: null};

  static getDerivedStateFromError(error: Error): State {
    return {error};
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('UI-Fehler:', error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    const lang = resolveLanguage('auto', navigator.language);
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 bg-slate-50 p-8 text-center text-slate-900 dark:bg-slate-900 dark:text-slate-100">
        <h1 className="text-xl font-semibold">{translate(lang, 'boundary.title')}</h1>
        <p className="max-w-md text-sm text-slate-500 dark:text-slate-400">{this.state.error.message}</p>
        <button className={btnPrimary} onClick={() => window.location.reload()}>
          {translate(lang, 'boundary.reload')}
        </button>
      </div>
    );
  }
}
