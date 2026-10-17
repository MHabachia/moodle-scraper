import {createContext, useContext, useMemo, type ReactNode} from 'react';
import {de, type Key} from './de';
import {en} from './en';

export type Language = 'de' | 'en';
export type LanguageSetting = 'auto' | Language;
export type Params = Record<string, string | number>;
export type Translator = (key: Key, params?: Params) => string;

const dictionaries: Record<Language, Record<Key, string>> = {de, en};

/** "auto" folgt der Systemsprache (Deutsch, sonst Englisch). */
export function resolveLanguage(setting: LanguageSetting, navigatorLanguage: string): Language {
  if (setting !== 'auto') return setting;
  return navigatorLanguage.toLowerCase().startsWith('de') ? 'de' : 'en';
}

/** Ersetzt `{name}`-Platzhalter. */
export function interpolate(text: string, params?: Params): string {
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m));
}

export function translate(lang: Language, key: Key, params?: Params): string {
  return interpolate(dictionaries[lang][key] ?? de[key], params);
}

const I18nContext = createContext<{lang: Language; t: Translator}>({
  lang: 'de',
  t: (key, params) => translate('de', key, params),
});

export function I18nProvider({setting, children}: {setting: LanguageSetting; children: ReactNode}) {
  const lang = resolveLanguage(setting, typeof navigator === 'undefined' ? 'de' : navigator.language);
  const value = useMemo(() => ({lang, t: ((key, params) => translate(lang, key, params)) as Translator}), [lang]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useT(): Translator {
  return useContext(I18nContext).t;
}

export function useLanguage(): Language {
  return useContext(I18nContext).lang;
}

/** Rendert `**fett**` als <b>. */
export function Rich({text}: {text: string}) {
  const parts = text.split('**');
  return (
    <>
      {parts.map((p, i) => (i % 2 === 1 ? <b key={i}>{p}</b> : <span key={i}>{p}</span>))}
    </>
  );
}

export type {Key};
