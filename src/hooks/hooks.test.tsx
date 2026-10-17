import {act, renderHook} from '@testing-library/react';
import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import type {Course, CourseContent, DownloadSummary} from '../api';
import {translate} from '../i18n';
import {DEFAULT_GLOBAL, DEFAULT_SITE, type Settings} from '../utils';
import {useAutoSync} from './useAutoSync';
import {useDownload} from './useDownload';

const mocks = vi.hoisted(() => ({
  scanCourse: vi.fn(),
  previewJob: vi.fn(),
  download: vi.fn(),
  cancelDownload: vi.fn(),
  openLogin: vi.fn(),
  checkLogin: vi.fn(),
}));

vi.mock('../api', async (orig) => {
  const m = await orig<typeof import('../api')>();
  return {
    ...m,
    api: {
      ...mocks,
      onProgress: vi.fn(() => Promise.resolve(() => {})),
    },
  };
});

const settings: Settings = {...DEFAULT_GLOBAL, ...DEFAULT_SITE, destDir: '/tmp/x', moodleUrl: 'https://m.de/'};
const t = (k: Parameters<typeof translate>[1], p?: Record<string, string | number>) => translate('de', k, p);

const course = (id: string, name: string): Course => ({
  id,
  name,
  shortName: '',
  url: '',
  category: '',
  progress: null,
  favourite: false,
  hidden: false,
  ended: false,
});

const content: CourseContent = {
  courseId: '1',
  sections: [
    {
      title: 'W1',
      number: 1,
      links: [{name: 'Video', url: 'https://youtu.be/x'}],
      items: [
        {id: '', name: 'a', filename: 'a.pdf', ext: 'pdf', url: 'u1', size: 10, kind: 'resource', folder: null},
        {id: '', name: 'x', filename: 'x.exe', ext: 'exe', url: 'u2', size: 10, kind: 'resource', folder: null},
      ],
    },
  ],
};

const okSummary = (o: Partial<DownloadSummary> = {}): DownloadSummary => ({
  cancelled: false,
  sessionExpired: false,
  results: [{courseId: '1', name: 'Kurs', path: '/tmp/x/Kurs', downloaded: 1, updated: 0, skipped: 0, bytes: 10, failed: []}],
  ...o,
});

function setup(over: Partial<Settings> = {}) {
  const onError = vi.fn();
  const onMessage = vi.fn();
  const onScanned = vi.fn();
  const hook = renderHook(
    (p: {s: Settings}) =>
      useDownload({settings: p.s, t, cache: {}, excluded: {}, onScanned, onError, onMessage}),
    {initialProps: {s: {...settings, ...over}}},
  );
  return {...hook, onError, onMessage, onScanned};
}

beforeEach(() => {
  Object.values(mocks).forEach((m) => m.mockReset());
  mocks.scanCourse.mockResolvedValue(content);
  mocks.download.mockResolvedValue(okSummary());
  mocks.previewJob.mockResolvedValue([{courseId: '1', name: 'Kurs', newFiles: 1, changedFiles: 0, unchanged: 0, blocked: 1, tooLarge: 0, bytes: 10, unknownSize: 0, links: 1}]);
});

describe('useDownload', () => {
  it('scans, shows a preview, then downloads after confirmation', async () => {
    const {result} = setup();
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    expect(result.current.phase).toBe('preview');
    expect(result.current.previews).toHaveLength(1);
    expect(mocks.download).not.toHaveBeenCalled();

    await act(async () => {
      await result.current.confirm();
    });
    expect(result.current.phase).toBe('done');
    const job = mocks.download.mock.calls[0][0];
    expect(job.courses[0].items.map((i: {filename: string}) => i.filename)).toEqual(['a.pdf']); // exe gefiltert
    expect(job.courses[0].links).toHaveLength(1);
    expect(job.saveLinks).toBe(true);
    expect(job.allowExecutables).toBe(false);
  });

  it('downloads directly when the preview is turned off', async () => {
    const {result} = setup({preview: false});
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    expect(mocks.previewJob).not.toHaveBeenCalled();
    expect(result.current.phase).toBe('done');
    expect(result.current.summary?.results[0].downloaded).toBe(1);
  });

  it('cancelling the preview does not download', async () => {
    const {result} = setup();
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    act(() => result.current.close());
    expect(result.current.phase).toBe('idle');
    expect(mocks.download).not.toHaveBeenCalled();
  });

  it('reports when no file matches the filters', async () => {
    mocks.scanCourse.mockResolvedValue({...content, sections: [{...content.sections[0], items: [], links: []}]});
    const {result, onMessage} = setup();
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    expect(onMessage).toHaveBeenCalledWith(t('msg.noFiles'));
    expect(result.current.phase).toBe('idle');
  });

  it('forwards an expired session while scanning', async () => {
    mocks.scanCourse.mockRejectedValue('SESSION_EXPIRED');
    const {result, onError} = setup();
    await act(async () => {
      await result.current.start([course('1', 'A'), course('2', 'B')]);
    });
    expect(onError).toHaveBeenCalledWith('SESSION_EXPIRED');
    expect(result.current.phase).toBe('idle');
  });

  it('skips unreadable courses but keeps the others', async () => {
    mocks.scanCourse.mockImplementation((id: string) => (id === '2' ? Promise.reject('HTTP 404') : Promise.resolve(content)));
    const {result} = setup({preview: false});
    await act(async () => {
      await result.current.start([course('1', 'A'), course('2', 'B')]);
    });
    expect(mocks.download.mock.calls[0][0].courses.map((c: {id: string}) => c.id)).toEqual(['1']);
  });

  it('retries only failed items', async () => {
    const item = {section: 'S', sectionIndex: 1, folder: null, filename: 'a.pdf', url: 'u1', size: 10};
    mocks.download.mockResolvedValueOnce(
      okSummary({results: [{courseId: '1', name: 'Kurs', path: '', downloaded: 0, updated: 0, skipped: 0, bytes: 0, failed: [{name: 'a.pdf', error: 'HTTP 500', item}]}]}),
    );
    const {result} = setup({preview: false});
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    mocks.download.mockResolvedValueOnce(okSummary());
    await act(async () => {
      await result.current.retry();
    });
    expect(mocks.download.mock.calls[1][0].courses).toEqual([{id: '1', name: 'Kurs', items: [item], links: []}]);
  });

  it('signs in again after an expired session and resumes', async () => {
    vi.useFakeTimers();
    const item = {section: 'S', sectionIndex: 1, folder: null, filename: 'a.pdf', url: 'u1', size: 10};
    mocks.download.mockResolvedValueOnce(
      okSummary({
        sessionExpired: true,
        results: [{courseId: '1', name: 'Kurs', path: '', downloaded: 0, updated: 0, skipped: 0, bytes: 0, failed: [{name: 'a.pdf', error: 'SESSION_EXPIRED', item}]}],
      }),
    );
    mocks.openLogin.mockResolvedValue('https://m.de/');
    mocks.checkLogin.mockResolvedValueOnce(null).mockResolvedValueOnce({baseUrl: 'https://m.de/', host: 'm.de', user: 'x'});
    const {result} = setup({preview: false});
    await act(async () => {
      await result.current.start([course('1', 'Kurs')]);
    });
    expect(result.current.summary?.sessionExpired).toBe(true);

    mocks.download.mockResolvedValueOnce(okSummary());
    let p!: Promise<void>;
    act(() => {
      p = result.current.reloginAndResume();
    });
    expect(result.current.relogin).toBe('waiting');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2600 * 2);
      await p;
    });
    expect(mocks.openLogin).toHaveBeenCalledWith('https://m.de/');
    expect(mocks.checkLogin).toHaveBeenCalledTimes(2);
    expect(mocks.download).toHaveBeenCalledTimes(2);
    expect(result.current.relogin).toBe('idle');
    expect(result.current.summary?.sessionExpired).toBe(false);
    vi.useRealTimers();
  });

  it('background sync runs silently and returns the summary', async () => {
    const {result} = setup();
    let res: DownloadSummary | null = null;
    await act(async () => {
      res = await result.current.sync([course('1', 'Kurs')]);
    });
    expect(res).not.toBeNull();
    expect(result.current.phase).toBe('idle');
    expect(mocks.scanCourse).toHaveBeenCalledWith('1', true); // frisch scannen
    expect(mocks.previewJob).not.toHaveBeenCalled();
  });
});

describe('useAutoSync', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('runs periodically while enabled', () => {
    const run = vi.fn();
    renderHook(() => useAutoSync(true, 2, run));
    vi.advanceTimersByTime(2 * 3_600_000 - 1);
    expect(run).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(run).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(2 * 3_600_000);
    expect(run).toHaveBeenCalledTimes(2);
  });

  it('does nothing when disabled and stops when switched off', () => {
    const run = vi.fn();
    const {rerender} = renderHook((p: {on: boolean}) => useAutoSync(p.on, 1, run), {initialProps: {on: false}});
    vi.advanceTimersByTime(5 * 3_600_000);
    expect(run).not.toHaveBeenCalled();
    rerender({on: true});
    vi.advanceTimersByTime(3_600_000);
    expect(run).toHaveBeenCalledTimes(1);
    rerender({on: false});
    vi.advanceTimersByTime(5 * 3_600_000);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('always calls the latest callback', () => {
    const a = vi.fn();
    const b = vi.fn();
    const {rerender} = renderHook((p: {cb: () => void}) => useAutoSync(true, 1, p.cb), {initialProps: {cb: a}});
    rerender({cb: b});
    vi.advanceTimersByTime(3_600_000);
    expect(a).not.toHaveBeenCalled();
    expect(b).toHaveBeenCalledTimes(1);
  });
});
