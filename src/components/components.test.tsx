import {cleanup, fireEvent, render, screen} from '@testing-library/react';
import {afterEach, describe, expect, it, vi} from 'vitest';
import type {Course, CoursePreview, DownloadSummary} from '../api';
import {I18nProvider} from '../i18n';
import {DEFAULT_GLOBAL, DEFAULT_SITE, type Settings} from '../utils';
import {ActionBar} from './ActionBar';
import {CourseCard} from './CourseCard';
import {dueLabel} from './DeadlinesDialog';
import {PreviewDialog, previewTotals} from './PreviewDialog';
import {ProgressDialog, updateSpeed} from './ProgressDialog';
import {SettingsDialog} from './SettingsDialog';
import {translate, type Translator} from '../i18n';

afterEach(cleanup);

const settings: Settings = {...DEFAULT_GLOBAL, ...DEFAULT_SITE, destDir: '/tmp/Moodle'};

const course: Course = {
  id: '1',
  name: 'Analysis 1',
  shortName: 'ANA1',
  url: '',
  category: 'Mathematik',
  progress: 40,
  favourite: true,
  hidden: false,
  ended: true,
};

const wrap = (ui: React.ReactElement, lang: 'de' | 'en' = 'de') => render(<I18nProvider setting={lang}>{ui}</I18nProvider>);

describe('CourseCard', () => {
  it('shows name, badges and toggles on click', () => {
    const onToggle = vi.fn();
    wrap(<CourseCard course={course} checked={false} excludedCount={2} onToggle={onToggle} onOpen={() => {}} />);
    expect(screen.getByText('Analysis 1')).toBeTruthy();
    expect(screen.getByText('Beendet')).toBeTruthy();
    expect(screen.getByText('2 Dateien abgewählt')).toBeTruthy();
    fireEvent.click(screen.getByText('Analysis 1'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('opens the file list without toggling the selection', () => {
    const onToggle = vi.fn();
    const onOpen = vi.fn();
    wrap(<CourseCard course={course} checked onToggle={onToggle} onOpen={onOpen} excludedCount={0} />);
    fireEvent.click(screen.getByTitle('Inhalt ansehen & Dateien auswählen'));
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onToggle).not.toHaveBeenCalled();
  });

  it('renders in english', () => {
    wrap(<CourseCard course={course} checked={false} excludedCount={0} onToggle={() => {}} onOpen={() => {}} />, 'en');
    expect(screen.getByText('Ended')).toBeTruthy();
  });
});

describe('ActionBar', () => {
  const base = {settings, busy: false, onOpenSettings: () => {}, onClear: () => {}, onDownload: () => {}};

  it('disables download without selection and shows the count when selected', () => {
    const {rerender} = wrap(<ActionBar {...base} selectedCount={0} />);
    expect((screen.getByText('Herunterladen').closest('button') as HTMLButtonElement).disabled).toBe(true);
    rerender(
      <I18nProvider setting="de">
        <ActionBar {...base} selectedCount={3} />
      </I18nProvider>,
    );
    expect(screen.getByText('3 Kurse ausgewählt')).toBeTruthy();
    expect((screen.getByText('3 herunterladen').closest('button') as HTMLButtonElement).disabled).toBe(false);
  });

  it('is disabled while busy and calls download otherwise', () => {
    const onDownload = vi.fn();
    wrap(<ActionBar {...base} selectedCount={1} onDownload={onDownload} />);
    fireEvent.click(screen.getByText('1 herunterladen'));
    expect(onDownload).toHaveBeenCalledTimes(1);
    cleanup();
    wrap(<ActionBar {...base} selectedCount={1} busy />);
    expect((screen.getByText('1 herunterladen').closest('button') as HTMLButtonElement).disabled).toBe(true);
  });
});

describe('SettingsDialog', () => {
  it('changes the folder structure preset and toggles options', () => {
    const onChange = vi.fn();
    wrap(<SettingsDialog settings={settings} onChange={onChange} onClose={() => {}} onEnableAutoSync={() => {}} />);
    fireEvent.click(screen.getByText('Abschnitte nummeriert'));
    expect(onChange).toHaveBeenCalledWith({dirTemplate: '{n} - {section}/{folder}'});
    fireEvent.click(screen.getByText('ZIP je Kurs', {selector: 'div'}));
    expect(onChange).toHaveBeenCalledWith({zip: true});
  });

  it('warns when programs are enabled and requests notification permission for auto sync', () => {
    const onChange = vi.fn();
    const onEnable = vi.fn();
    const withPrograms = {...settings, types: [...settings.types, 'programs']};
    wrap(<SettingsDialog settings={withPrograms} onChange={onChange} onClose={() => {}} onEnableAutoSync={onEnable} />);
    expect(screen.getByText(/Programme, Skripte und Makro-Dokumente/)).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Automatisch synchronisieren', {exact: false}));
    expect(onChange).toHaveBeenCalledWith({autoSync: true});
    expect(onEnable).toHaveBeenCalled();
  });
});

describe('PreviewDialog', () => {
  const p = (o: Partial<CoursePreview>): CoursePreview => ({
    courseId: '1',
    name: 'Kurs',
    newFiles: 0,
    changedFiles: 0,
    unchanged: 0,
    blocked: 0,
    tooLarge: 0,
    bytes: 0,
    unknownSize: 0,
    links: 0,
    ...o,
  });

  it('computes totals', () => {
    const t = previewTotals([p({newFiles: 2, changedFiles: 1, bytes: 100}), p({newFiles: 1, links: 2, bytes: 50})]);
    expect(t).toEqual({files: 4, bytes: 150, links: 2, nothing: false});
    expect(previewTotals([p({unchanged: 9})]).nothing).toBe(true);
  });

  it('confirm is disabled when nothing is new', () => {
    wrap(<PreviewDialog previews={[p({unchanged: 5})]} onConfirm={() => {}} onCancel={() => {}} />);
    expect(screen.getByText('Nichts Neues – alles ist aktuell.')).toBeTruthy();
    expect((screen.getByText('Herunterladen').closest('button') as HTMLButtonElement).disabled).toBe(true);
  });

  it('confirms when there is something to download', () => {
    const onConfirm = vi.fn();
    wrap(<PreviewDialog previews={[p({newFiles: 3, bytes: 2048})]} onConfirm={onConfirm} onCancel={() => {}} />);
    fireEvent.click(screen.getByText('Herunterladen'));
    expect(onConfirm).toHaveBeenCalled();
    expect(screen.getByText(/Gesamt: 3 Dateien/)).toBeTruthy();
  });
});

describe('ProgressDialog', () => {
  const summary = (o: Partial<DownloadSummary>, failed: number): DownloadSummary => ({
    cancelled: false,
    sessionExpired: false,
    results: [
      {
        courseId: '1',
        name: 'Kurs A',
        path: '/tmp/a',
        downloaded: 2,
        updated: 1,
        skipped: 3,
        bytes: 2048,
        failed: Array.from({length: failed}, (_, i) => ({name: `f${i}.pdf`, error: 'HTTP 500', item: null})),
      },
    ],
    ...o,
  });
  const base = {
    phase: 'done' as const,
    scan: {index: 0, total: 0, name: '', preparing: false},
    progress: null,
    relogin: 'idle' as const,
    onRetry: () => {},
    onRelogin: () => {},
    onCancel: () => {},
    onClose: () => {},
  };

  it('summarizes results and offers retry for failures', () => {
    const onRetry = vi.fn();
    wrap(<ProgressDialog {...base} summary={summary({}, 2)} onRetry={onRetry} />);
    expect(screen.getAllByText(/2 neu · 1 aktualisiert/).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByText('Fehlgeschlagene wiederholen'));
    expect(onRetry).toHaveBeenCalled();
  });

  it('shows the session-expired banner with a resume button instead of plain retry', () => {
    const onRelogin = vi.fn();
    wrap(<ProgressDialog {...base} summary={summary({sessionExpired: true}, 2)} onRelogin={onRelogin} />);
    expect(screen.getByText('Anmeldung abgelaufen')).toBeTruthy();
    expect(screen.queryByText('Fehlgeschlagene wiederholen')).toBeNull();
    fireEvent.click(screen.getByText('Erneut anmelden & fortsetzen'));
    expect(onRelogin).toHaveBeenCalled();
  });

  it('shows progress with files and speed while downloading', () => {
    wrap(
      <ProgressDialog
        {...base}
        phase="downloading"
        summary={null}
        progress={{
          courseIndex: 0,
          courseCount: 2,
          courseName: 'Kurs A',
          filesDone: 3,
          filesTotal: 10,
          bytesDone: 1024,
          active: ['a.pdf', 'b.pdf'],
        }}
      />,
    );
    expect(screen.getByText('3 / 10 Dateien')).toBeTruthy();
    expect(screen.getByText('a.pdf')).toBeTruthy();
  });
});

describe('helpers', () => {
  it('smooths download speed', () => {
    let s = updateSpeed({t: 0, bytes: 0, speed: 0}, 0, 1000);
    expect(s.speed).toBe(0);
    s = updateSpeed(s, 100, 1100); // zu früh: unverändert
    expect(s.bytes).toBe(0);
    s = updateSpeed(s, 1000, 2000);
    expect(Math.round(s.speed)).toBe(1000);
    s = updateSpeed(s, 3000, 3000);
    expect(s.speed).toBeGreaterThan(1000);
    expect(updateSpeed(s, 10, 4000).speed).toBe(0); // Zähler zurückgesetzt
  });

  it('describes due dates', () => {
    const t: Translator = (k, p) => translate('de', k, p);
    const now = 1_000_000;
    expect(dueLabel(t, now - 10, now)).toBe('überfällig');
    expect(dueLabel(t, now + 3600, now)).toBe('heute');
    expect(dueLabel(t, now + 86_400 + 5, now)).toBe('morgen');
    expect(dueLabel(t, now + 3 * 86_400 + 5, now)).toBe('in 3 Tagen');
  });
});
