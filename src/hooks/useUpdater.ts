import {useCallback, useEffect, useRef, useState} from 'react';
import {errorMessage} from '../api';

export type UpdateState =
  | {status: 'idle' | 'checking' | 'none'}
  | {status: 'available' | 'installing'; version: string}
  | {status: 'error'; message: string};

/** Wartezeit, damit die Suche den Start der App nicht ausbremst */
const AUTO_CHECK_DELAY_MS = 8000;

interface PendingUpdate {
  version: string;
  downloadAndInstall: () => Promise<void>;
}

/**
 * Sucht auf GitHub nach einer signierten neuen Version und installiert sie auf Wunsch.
 * Die automatische Suche bleibt still (offline, kein Release); eine manuelle meldet Fehler.
 */
export function useUpdater(autoCheck: boolean) {
  const [state, setState] = useState<UpdateState>({status: 'idle'});
  const pending = useRef<PendingUpdate | null>(null);

  const check = useCallback(async (manual: boolean) => {
    setState({status: 'checking'});
    try {
      const {check: checkForUpdate} = await import('@tauri-apps/plugin-updater');
      const update = await checkForUpdate();
      if (update) {
        pending.current = update;
        setState({status: 'available', version: update.version});
      } else {
        setState({status: 'none'});
      }
    } catch (e) {
      setState(manual ? {status: 'error', message: errorMessage(e)} : {status: 'idle'});
    }
  }, []);

  const install = useCallback(async () => {
    const update = pending.current;
    if (!update) return;
    setState({status: 'installing', version: update.version});
    try {
      await update.downloadAndInstall();
      const {relaunch} = await import('@tauri-apps/plugin-process');
      await relaunch();
    } catch (e) {
      setState({status: 'error', message: errorMessage(e)});
    }
  }, []);

  useEffect(() => {
    if (!autoCheck) return;
    const timer = setTimeout(() => void check(false), AUTO_CHECK_DELAY_MS);
    return () => clearTimeout(timer);
  }, [check, autoCheck]);

  return {state, checkNow: () => check(true), install};
}
