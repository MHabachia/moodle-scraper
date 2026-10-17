import {useCallback, useEffect, useMemo, useState} from 'react';
import {ActionBar} from './components/ActionBar';
import {ConnectScreen} from './components/ConnectScreen';
import {ContentDialog} from './components/ContentDialog';
import {CourseList} from './components/CourseList';
import {CourseToolbar} from './components/CourseToolbar';
import {DeadlinesDialog} from './components/DeadlinesDialog';
import {LoginWaiting} from './components/LoginWaiting';
import {PreviewDialog} from './components/PreviewDialog';
import {ProgressDialog} from './components/ProgressDialog';
import {SettingsDialog} from './components/SettingsDialog';
import {TopBar} from './components/TopBar';
import {api, errorMessage, LOGIN_WINDOW_CLOSED, SESSION_EXPIRED, type Course, type CourseContent, type SessionInfo} from './api';
import {useAutoSync} from './hooks/useAutoSync';
import {useDownload} from './hooks/useDownload';
import {useT} from './i18n';
import {countByFilter, visibleCourses} from './lib/courses';
import {ensureNotificationPermission, notify} from './notify';
import type {Filter, Sort, Stage} from './types';
import type {Settings} from './utils';

interface Props {
  settings: Settings;
  setSettings: (patch: Partial<Settings>) => void;
  session: SessionInfo | null;
  setSession: (s: SessionInfo | null) => void;
}

export function AppView({settings, setSettings, session, setSession}: Props) {
  const t = useT();
  const [stage, setStage] = useState<Stage>(settings.rememberLogin && settings.moodleUrl ? 'restoring' : 'connect');
  const [error, setError] = useState('');
  const [courses, setCourses] = useState<Course[]>([]);
  const [loadingCourses, setLoadingCourses] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [sort, setSort] = useState<Sort>('name');
  const [showSettings, setShowSettings] = useState(false);
  const [showDeadlines, setShowDeadlines] = useState(false);
  const [detail, setDetail] = useState<Course | null>(null);
  const [cache, setCache] = useState<Record<string, CourseContent>>({});
  const [excluded, setExcluded] = useState<Record<string, string[]>>({});

  const backToConnect = useCallback(
    (msg = '') => {
      setStage('connect');
      setSession(null);
      setCourses([]);
      setSelected(new Set());
      setCache({});
      setExcluded({});
      setError(msg);
    },
    [setSession],
  );

  const handleError = useCallback(
    (e: unknown) => {
      const m = errorMessage(e);
      if (m === SESSION_EXPIRED) backToConnect(t('msg.sessionExpired'));
      else setError(m);
    },
    [backToConnect, t],
  );

  const dl = useDownload({
    settings,
    t,
    cache,
    excluded,
    onScanned: (id, content) => setCache((c) => ({...c, [id]: content})),
    onError: handleError,
    onMessage: setError,
  });

  // Kursliste laden (zuletzt gewählte Kurse vorauswählen)
  const loadCourses = useCallback(
    async (refresh = false) => {
      setLoadingCourses(true);
      setError('');
      try {
        const list = await api.listCourses();
        setCourses(list);
        if (refresh) setCache({});
        setSelected((prev) => {
          if (prev.size > 0) return new Set([...prev].filter((id) => list.some((k) => k.id === id)));
          return new Set(settings.lastSelected.filter((id) => list.some((k) => k.id === id)));
        });
      } catch (e) {
        handleError(e);
      } finally {
        setLoadingCourses(false);
      }
    },
    [handleError, settings.lastSelected],
  );

  const onSignedIn = useCallback(
    (info: SessionInfo) => {
      setSession(info);
      setStage('courses');
      void loadCourses();
    },
    [setSession, loadCourses],
  );

  // Gespeicherte Anmeldung wiederherstellen (einmalig beim Start)
  useEffect(() => {
    if (stage !== 'restoring') return;
    let alive = true;
    api
      .restoreSession(settings.moodleUrl)
      .then((info) => alive && (info ? onSignedIn(info) : setStage('connect')))
      .catch(() => alive && setStage('connect'));
    return () => {
      alive = false;
    };
    // bewusst nur beim ersten Rendern
  }, []);

  // Verbinden / Login
  const startLogin = async () => {
    setError('');
    try {
      const base = await api.openLogin(settings.moodleUrl);
      setSettings({
        moodleUrl: base,
        recentUrls: [base, ...settings.recentUrls.filter((u) => u !== base)].slice(0, 5),
      });
      setStage('login');
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  // Login-Fenster beobachten, bis die Session steht
  useEffect(() => {
    if (stage !== 'login') return;
    let alive = true;
    let busy = false;
    const tick = async () => {
      if (busy || !alive) return;
      busy = true;
      try {
        const info = await api.checkLogin();
        if (info && alive) onSignedIn(info);
      } catch (e) {
        if (alive && errorMessage(e) === LOGIN_WINDOW_CLOSED) backToConnect(t('msg.loginClosed'));
      } finally {
        busy = false;
      }
    };
    const id = setInterval(tick, 2500);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [stage, onSignedIn, backToConnect, t]);

  const logout = async () => {
    await api.logout().catch(() => {});
    backToConnect('');
  };

  // Kursliste
  const counts = useMemo(() => countByFilter(courses), [courses]);
  const visible = useMemo(() => visibleCourses(courses, search, filter, sort), [courses, search, filter, sort]);
  const allVisibleSelected = visible.length > 0 && visible.every((k) => selected.has(k.id));

  const toggle = (id: string) =>
    setSelected((prev) => {
      const n = new Set(prev);
      if (!n.delete(id)) n.add(id);
      return n;
    });

  const toggleAllVisible = () =>
    setSelected((prev) => {
      const n = new Set(prev);
      visible.forEach((k) => (allVisibleSelected ? n.delete(k.id) : n.add(k.id)));
      return n;
    });

  const selectActive = () => setSelected(new Set(courses.filter((k) => !k.ended && !k.hidden).map((k) => k.id)));

  const download = () => {
    if (!settings.destDir) return setShowSettings(true);
    setError('');
    setSettings({lastSelected: [...selected]});
    void dl.start(courses.filter((k) => selected.has(k.id)));
  };

  // Auto-Sync (stiller Lauf, Benachrichtigung bei Neuem)
  const runAutoSync = useCallback(async () => {
    const chosen = courses.filter((k) => settings.lastSelected.includes(k.id));
    try {
      const res = await dl.sync(chosen);
      if (!res) return;
      if (res.sessionExpired) return void notify('Moodle Scraper', t('msg.syncRelogin'));
      const n = res.results.reduce((a, r) => a + r.downloaded + r.updated, 0);
      if (n > 0) void notify('Moodle Scraper', t('msg.syncNotify', {n}));
    } catch (e) {
      if (errorMessage(e) === SESSION_EXPIRED) void notify('Moodle Scraper', t('msg.syncRelogin'));
    }
  }, [courses, settings.lastSelected, dl, t]);

  useAutoSync(stage === 'courses' && settings.autoSync, settings.autoSyncHours, runAutoSync);

  const step = dl.phase !== 'idle' ? 4 : stage === 'connect' || stage === 'restoring' ? 1 : stage === 'login' ? 2 : 3;

  return (
    <div className="flex h-full flex-col bg-slate-50 text-slate-900 dark:bg-slate-900 dark:text-slate-100">
      <TopBar
        step={step}
        theme={settings.theme}
        onTheme={(th) => setSettings({theme: th})}
        session={stage === 'courses' ? session : null}
        loading={loadingCourses}
        onRefresh={() => loadCourses(true)}
        onSettings={() => setShowSettings(true)}
        onDeadlines={() => setShowDeadlines(true)}
        onLogout={logout}
      />

      {(stage === 'connect' || stage === 'restoring') && (
        <ConnectScreen
          url={settings.moodleUrl}
          recent={settings.recentUrls}
          error={error}
          remember={settings.rememberLogin}
          restoring={stage === 'restoring'}
          onUrl={(u) => setSettings({moodleUrl: u})}
          onRemember={(v) => setSettings({rememberLogin: v})}
          onStart={startLogin}
        />
      )}

      {stage === 'login' && <LoginWaiting error={error} onReopen={startLogin} onCancel={() => backToConnect('')} />}

      {stage === 'courses' && session && (
        <>
          <CourseToolbar
            search={search}
            onSearch={setSearch}
            sort={sort}
            onSort={setSort}
            filter={filter}
            onFilter={setFilter}
            counts={counts}
            allSelected={allVisibleSelected}
            onToggleAll={toggleAllVisible}
            onSelectActive={selectActive}
            showHint={selected.size === 0}
          />
          <CourseList
            courses={courses}
            visible={visible}
            loading={loadingCourses}
            error={error}
            selected={selected}
            excluded={excluded}
            onToggle={toggle}
            onOpen={setDetail}
          />
          <ActionBar
            selectedCount={selected.size}
            settings={settings}
            busy={dl.phase !== 'idle'}
            onOpenSettings={() => setShowSettings(true)}
            onClear={() => setSelected(new Set())}
            onDownload={download}
          />
        </>
      )}

      {showSettings && (
        <SettingsDialog
          settings={settings}
          onChange={setSettings}
          onClose={() => setShowSettings(false)}
          onEnableAutoSync={() => void ensureNotificationPermission()}
        />
      )}

      {showDeadlines && <DeadlinesDialog onError={handleError} onClose={() => setShowDeadlines(false)} />}

      {detail && (
        <ContentDialog
          course={detail}
          content={cache[detail.id]}
          excluded={excluded[detail.id] ?? []}
          types={settings.types}
          onLoaded={(c) => setCache((p) => ({...p, [detail.id]: c}))}
          onExclude={(list) => setExcluded((p) => ({...p, [detail.id]: list}))}
          onError={handleError}
          onClose={() => setDetail(null)}
        />
      )}

      {dl.phase === 'preview' && (
        <PreviewDialog previews={dl.previews} onConfirm={() => void dl.confirm()} onCancel={dl.close} />
      )}

      {dl.phase !== 'idle' && dl.phase !== 'preview' && (
        <ProgressDialog
          phase={dl.phase}
          scan={dl.scanInfo}
          progress={dl.progress}
          summary={dl.summary}
          relogin={dl.relogin}
          onRetry={dl.retry}
          onRelogin={dl.reloginAndResume}
          onCancel={dl.cancel}
          onClose={dl.close}
        />
      )}
    </div>
  );
}
