import {useState} from 'react';
import {AppView} from './AppView';
import type {SessionInfo} from './api';
import {useSettings} from './hooks/useSettings';
import {I18nProvider} from './i18n';

/** Wurzel: hält Session und Einstellungen und stellt die Sprache bereit. */
export default function App() {
  const [session, setSession] = useState<SessionInfo | null>(null);
  const {settings, setSettings} = useSettings(session?.host ?? '');
  return (
    <I18nProvider setting={settings.language}>
      <AppView settings={settings} setSettings={setSettings} session={session} setSession={setSession} />
    </I18nProvider>
  );
}
