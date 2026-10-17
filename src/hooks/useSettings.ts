import {useCallback, useEffect, useMemo, useState} from 'react';
import {api} from '../api';
import {loadSettings, resolveSite, saveSettings, splitPatch, type Settings, type Stored} from '../utils';

/** Einstellungen: global + Profil der aktuellen Moodle-Instanz (`host`); wendet das Theme an. */
export function useSettings(host: string) {
  const [stored, setStored] = useState<Stored>(loadSettings);

  const settings: Settings = useMemo(
    () => ({
      moodleUrl: stored.moodleUrl,
      recentUrls: stored.recentUrls,
      theme: stored.theme,
      language: stored.language,
      rememberLogin: stored.rememberLogin,
      background: stored.background,
      ...resolveSite(stored, host),
    }),
    [stored, host],
  );

  const setSettings = useCallback(
    (patch: Partial<Settings>) => {
      const {global, site} = splitPatch(patch);
      setStored((prev) => {
        const next: Stored = {
          ...prev,
          ...global,
          profiles: Object.keys(site).length
            ? {...prev.profiles, [host]: {...prev.profiles[host], ...site}}
            : prev.profiles,
        };
        saveSettings(next);
        return next;
      });
    },
    [host],
  );

  // Standard-Zielordner
  useEffect(() => {
    if (!settings.destDir) {
      api
        .defaultDir()
        .then((d) => d && setSettings({destDir: d}))
        .catch(() => {});
    }
  }, [settings.destDir, setSettings]);

  // Theme (System / Hell / Dunkel)
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      const dark = settings.theme === 'dark' || (settings.theme === 'system' && mq.matches);
      document.documentElement.classList.toggle('dark', dark);
      document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
    };
    apply();
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [settings.theme]);

  // Backend über Hintergrundbetrieb und "Angemeldet bleiben" informieren
  useEffect(() => {
    api.setBackground(settings.background).catch(() => {});
  }, [settings.background]);
  useEffect(() => {
    api.setRemember(settings.rememberLogin, settings.moodleUrl || undefined).catch(() => {});
  }, [settings.rememberLogin, settings.moodleUrl]);

  return {settings, setSettings};
}
