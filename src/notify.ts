import {isPermissionGranted, requestPermission, sendNotification} from '@tauri-apps/plugin-notification';

/** Fragt (einmalig) die Erlaubnis für Systembenachrichtigungen ab. */
export async function ensureNotificationPermission(): Promise<boolean> {
  try {
    return (await isPermissionGranted()) || (await requestPermission()) === 'granted';
  } catch {
    return false;
  }
}

/** Sendet eine Benachrichtigung, falls erlaubt; Fehler werden ignoriert. */
export async function notify(title: string, body: string): Promise<void> {
  try {
    if (await isPermissionGranted()) sendNotification({title, body});
  } catch {
    /* Benachrichtigungen sind optional */
  }
}
