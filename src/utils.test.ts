import {describe, expect, it} from 'vitest';
import type {Course, CourseContent, DownloadSummary} from './api';
import {countByFilter, visibleCourses} from './lib/courses';
import {makeJob, retryCourses} from './lib/job';
import {
  buildCourseItems,
  categoryOf,
  cleanCourseName,
  courseFolderName,
  DEFAULT_GLOBAL,
  DEFAULT_SITE,
  DEFAULT_TYPES,
  migrateStored,
  migrateV1,
  SETTINGS_VERSION,
  buildCourseLinks,
  formatBytes,
  itemAllowed,
  resolveSite,
  splitPatch,
  type Stored,
} from './utils';

describe('formatBytes', () => {
  it('formats units', () => {
    expect(formatBytes(null)).toBe('–');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
    expect(formatBytes(250 * 1024 * 1024)).toBe('250 MB');
  });
});

describe('categoryOf', () => {
  it('maps extensions and falls back to other', () => {
    expect(categoryOf('PDF')).toBe('docs');
    expect(categoryOf('pptx')).toBe('slides');
    expect(categoryOf('ipynb')).toBe('code');
    expect(categoryOf('xyz')).toBe('other');
    expect(categoryOf('')).toBe('other');
  });
});

describe('programs category', () => {
  it('is off by default and detects executables', () => {
    expect(DEFAULT_TYPES).not.toContain('programs');
    expect(DEFAULT_TYPES).toContain('other');
    for (const e of ['exe', 'sh', 'docm', 'jar', 'dmg']) expect(categoryOf(e)).toBe('programs');
    expect(categoryOf('py')).toBe('code');
  });
});

describe('course naming', () => {
  it('removes semester prefixes and suffixes', () => {
    expect(cleanCourseName('WiSe 24/25 - Analysis 1')).toBe('Analysis 1');
    expect(cleanCourseName('SoSe2024: Physik')).toBe('Physik');
    expect(cleanCourseName('[WS 2023/24] Informatik')).toBe('Informatik');
    expect(cleanCourseName('2024/25 - Chemie')).toBe('Chemie');
    expect(cleanCourseName('Mathe (WiSe 23/24)')).toBe('Mathe');
    expect(cleanCourseName('Analysis 1')).toBe('Analysis 1');
  });

  it('never returns an empty name', () => {
    expect(cleanCourseName('WiSe 24/25')).toBe('WiSe 24/25');
  });

  it('respects the naming mode', () => {
    const c = {name: 'WiSe 24/25 - Analysis 1', shortName: 'ANA1'};
    expect(courseFolderName(c, 'full')).toBe('WiSe 24/25 - Analysis 1');
    expect(courseFolderName(c, 'clean')).toBe('Analysis 1');
    expect(courseFolderName(c, 'short')).toBe('ANA1');
    expect(courseFolderName({...c, shortName: ' '}, 'short')).toBe('Analysis 1');
  });
});

describe('buildCourseItems', () => {
  const content: CourseContent = {
    courseId: '1',
    sections: [
      {
        title: 'Woche 1',
        number: 1,
        links: [{name: 'Video', url: 'https://youtu.be/x'}],
        items: [
          {id: '', name: 'a', filename: 'a.pdf', ext: 'pdf', url: 'u1', size: 1000, kind: 'resource', folder: null},
          {id: '', name: 'b', filename: 'b.mp4', ext: 'mp4', url: 'u2', size: 300 * 1024 * 1024, kind: 'resource', folder: null},
          {id: '', name: 'c', filename: 'c.pptx', ext: 'pptx', url: 'u3', size: null, kind: 'folder', folder: 'Folien/Teil A'},
        ],
      },
    ],
  };

  it('filters by type, size and exclusion', () => {
    const all = buildCourseItems(content, {excluded: new Set(), types: ['docs', 'slides', 'media'], maxSizeMb: 0});
    expect(all.map((i) => i.url)).toEqual(['u1', 'u2', 'u3']);

    const limited = buildCourseItems(content, {excluded: new Set(), types: ['docs', 'slides', 'media'], maxSizeMb: 100});
    expect(limited.map((i) => i.url)).toEqual(['u1', 'u3']); // 300 MB entfällt, unbekannte Größe bleibt

    const noSlides = buildCourseItems(content, {excluded: new Set(['u1']), types: ['docs', 'media'], maxSizeMb: 0});
    expect(noSlides.map((i) => i.url)).toEqual(['u2']);
  });

  it('carries section metadata and folder', () => {
    const [first, , third] = buildCourseItems(content, {excluded: new Set(), types: ['docs', 'slides', 'media'], maxSizeMb: 0});
    expect(first.section).toBe('Woche 1');
    expect(first.sectionIndex).toBe(1);
    expect(third.folder).toBe('Folien/Teil A');
  });

  it('itemAllowed uses the extension category', () => {
    const it = content.sections[0].items[0];
    expect(itemAllowed(it, ['docs'])).toBe(true);
    expect(itemAllowed(it, ['slides'])).toBe(false);
  });
});

describe('profiles', () => {
  const stored: Stored = {
    ...DEFAULT_GLOBAL,
    version: SETTINGS_VERSION,
    profiles: {'': {destDir: '/tmp/default', parallel: 2}, 'moodle.a.de': {parallel: 5, zip: true}},
  };

  it('merges defaults, template and host profile', () => {
    const a = resolveSite(stored, 'moodle.a.de');
    expect(a.destDir).toBe('/tmp/default');
    expect(a.parallel).toBe(5);
    expect(a.zip).toBe(true);
    expect(a.incremental).toBe(DEFAULT_SITE.incremental);

    const b = resolveSite(stored, 'moodle.b.de');
    expect(b.parallel).toBe(2);
    expect(b.zip).toBe(false);
  });

  it('splits patches into global and site settings', () => {
    const {global, site} = splitPatch({theme: 'dark', zip: true, moodleUrl: 'x'});
    expect(global).toEqual({theme: 'dark', moodleUrl: 'x'});
    expect(site).toEqual({zip: true});
  });
});


const course = (o: Partial<Course>): Course => ({
  id: '1',
  name: 'Kurs',
  shortName: '',
  url: '',
  category: '',
  progress: null,
  favourite: false,
  hidden: false,
  ended: false,
  ...o,
});

describe('course list', () => {
  const list = [
    course({id: '1', name: 'Zeta', progress: 10}),
    course({id: '2', name: 'alpha', favourite: true, progress: 90}),
    course({id: '3', name: 'Beta', ended: true, shortName: 'BETA-1'}),
    course({id: '4', name: 'Gamma', hidden: true}),
  ];

  it('counts per filter', () => {
    expect(countByFilter(list)).toEqual({all: 4, active: 2, favourite: 1, ended: 1, hidden: 1});
  });

  it('filters, searches and sorts', () => {
    expect(visibleCourses(list, '', 'all', 'name').map((c) => c.id)).toEqual(['2', '3', '4', '1']);
    expect(visibleCourses(list, '', 'active', 'progress').map((c) => c.id)).toEqual(['2', '1']);
    expect(visibleCourses(list, 'beta-1', 'all', 'name').map((c) => c.id)).toEqual(['3']);
    expect(visibleCourses(list, '', 'hidden', 'name').map((c) => c.id)).toEqual(['4']);
  });
});

describe('job building', () => {
  const site = {...DEFAULT_SITE, destDir: '/tmp/x', maxSizeMb: 100, types: [...DEFAULT_TYPES, 'programs']};
  const settings = {...DEFAULT_GLOBAL, ...site};

  it('maps settings to a backend job', () => {
    const job = makeJob(settings, []);
    expect(job.destDir).toBe('/tmp/x');
    expect(job.maxBytes).toBe(100 * 1024 * 1024);
    expect(job.allowExecutables).toBe(true);
    expect(makeJob({...settings, maxSizeMb: 0, types: DEFAULT_TYPES}, []).maxBytes).toBeNull();
    expect(makeJob({...settings, types: DEFAULT_TYPES}, []).allowExecutables).toBe(false);
  });

  it('builds a retry job from failures only', () => {
    const item = {section: 'S', sectionIndex: 1, folder: null, filename: 'a.pdf', url: 'u', size: null};
    const summary: DownloadSummary = {
      cancelled: false,
      sessionExpired: false,
      results: [
        {courseId: '1', name: 'A', path: '', downloaded: 1, updated: 0, skipped: 0, bytes: 1,
         failed: [{name: 'a.pdf', error: 'x', item}, {name: 'ZIP', error: 'y', item: null}]},
        {courseId: '2', name: 'B', path: '', downloaded: 1, updated: 0, skipped: 0, bytes: 1, failed: []},
      ],
    };
    expect(retryCourses(summary)).toEqual([{id: '1', name: 'A', items: [item], links: []}]);
  });
});

describe('links', () => {
  it('collects links of all sections', () => {
    const content: CourseContent = {
      courseId: '1',
      sections: [
        {title: 'A', number: 0, items: [], links: [{name: 'Vid', url: 'https://x.de/v'}]},
        {title: 'B', number: 1, items: [], links: []},
      ],
    };
    expect(buildCourseLinks(content)).toEqual([{section: 'A', name: 'Vid', url: 'https://x.de/v'}]);
  });
});

describe('settings migration', () => {
  it('fills defaults for old/partial data', () => {
    const m = migrateStored({moodleUrl: 'https://m.de/', theme: 'dark', profiles: {'m.de': {zip: true}}});
    expect(m.version).toBe(SETTINGS_VERSION);
    expect(m.theme).toBe('dark');
    expect(m.language).toBe('auto');
    expect(m.rememberLogin).toBe(false);
    expect(m.profiles['m.de']).toEqual({zip: true});
  });

  it('survives garbage input', () => {
    for (const bad of [null, undefined, 42, 'x', [], {profiles: 'kaputt'}]) {
      const m = migrateStored(bad);
      expect(m.version).toBe(SETTINGS_VERSION);
      expect(typeof m.profiles).toBe('object');
    }
  });

  it('migrates the v1 flat format into the template profile', () => {
    const m = migrateV1({moodleUrl: 'https://old.de/', destDir: '/d', zip: true, types: ['docs'], theme: 'light'});
    expect(m.moodleUrl).toBe('https://old.de/');
    expect(m.theme).toBe('light');
    expect(m.profiles['']).toEqual({destDir: '/d', zip: true, types: ['docs']});
  });

  it('new site defaults are sensible', () => {
    expect(DEFAULT_SITE.preview).toBe(true);
    expect(DEFAULT_SITE.autoSync).toBe(false);
    expect(DEFAULT_SITE.saveLinks).toBe(true);
  });
});
