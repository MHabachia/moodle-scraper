import type {CourseContent, DownloadItem, Item, JobLink} from './api';

export function formatBytes(n: number | null | undefined): string {
  if (n == null) return '–';
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
}

export interface Category {
  id: string;
  exts: string[];
}

export const CATEGORIES: Category[] = [
  {id: 'docs', exts: ['pdf', 'doc', 'docx', 'odt', 'rtf', 'txt', 'md', 'tex', 'epub']},
  {id: 'slides', exts: ['ppt', 'pptx', 'odp', 'key', 'pps', 'ppsx']},
  {id: 'sheets', exts: ['xls', 'xlsx', 'ods', 'csv']},
  {
    id: 'code',
    exts: ['py', 'ipynb', 'java', 'c', 'cpp', 'h', 'js', 'ts', 'r', 'm', 'sql', 'json', 'xml', 'html', 'htm', 'cs', 'rs', 'go'],
  },
  {id: 'archives', exts: ['zip', 'rar', '7z', 'tar', 'gz', 'tgz']},
  {id: 'images', exts: ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp']},
  {id: 'media', exts: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'mp3', 'wav', 'm4a', 'ogg']},
  {
    id: 'programs',
    exts: [
      'exe', 'msi', 'bat', 'cmd', 'com', 'scr', 'ps1', 'vbs', 'vbe', 'wsf', 'lnk', 'reg', 'dll', 'sh', 'command',
      'app', 'dmg', 'pkg', 'jar', 'apk', 'deb', 'rpm', 'run', 'bin', 'appimage', 'docm', 'xlsm', 'pptm', 'dotm',
      'xlam', 'ppam',
    ],
  },
  {id: 'other', exts: []},
];

/** Standardmäßig an: alles außer Programme/Makros (nur auf ausdrücklichen Wunsch). */
export const DEFAULT_TYPES = CATEGORIES.map((c) => c.id).filter((id) => id !== 'programs');

export function categoryOf(ext: string): string {
  const e = ext.toLowerCase();
  return CATEGORIES.find((c) => c.exts.includes(e))?.id ?? 'other';
}

export function itemAllowed(item: Item, types: string[]): boolean {
  return types.includes(categoryOf(item.ext));
}

export function hueOf(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
  return h;
}

// Einstellungen: global + je Moodle-Instanz (Profil)

export type CourseNaming = 'full' | 'short' | 'clean';

export interface SiteSettings {
  destDir: string;
  zip: boolean;
  /** Unterordner-Muster im Kursordner. Tokens: {section} {n} {folder} */
  dirTemplate: string;
  types: string[];
  incremental: boolean;
  parallel: number;
  /** 0 = unbegrenzt */
  maxSizeMb: number;
  courseNaming: CourseNaming;
  /** Externe Links je Kurs als Links.md speichern */
  saveLinks: boolean;
  /** Vorschau (neu/geändert) vor dem Download zeigen */
  preview: boolean;
  /** Automatisch synchronisieren (App muss laufen) */
  autoSync: boolean;
  autoSyncHours: number;
  /** Zuletzt gewählte Kurs-IDs (werden beim nächsten Start vorausgewählt) */
  lastSelected: string[];
}

export interface GlobalSettings {
  moodleUrl: string;
  recentUrls: string[];
  theme: 'system' | 'light' | 'dark';
  language: 'auto' | 'de' | 'en';
  /** Session im Passwortspeicher des Systems speichern */
  rememberLogin: boolean;
  /** Beim Schließen in den Tray minimieren */
  background: boolean;
}

export type Settings = GlobalSettings & SiteSettings;

export interface Stored extends GlobalSettings {
  /** Schema-Version der gespeicherten Einstellungen (für Migrationen) */
  version: number;
  /** Schlüssel = Host der Moodle-Instanz; '' = Vorlage für neue Instanzen */
  profiles: Record<string, Partial<SiteSettings>>;
}

export const DEFAULT_SITE: SiteSettings = {
  destDir: '',
  zip: false,
  dirTemplate: '{section}/{folder}',
  types: DEFAULT_TYPES,
  incremental: true,
  parallel: 3,
  maxSizeMb: 0,
  courseNaming: 'full',
  saveLinks: true,
  preview: true,
  autoSync: false,
  autoSyncHours: 3,
  lastSelected: [],
};

export const SETTINGS_VERSION = 3;

export const DEFAULT_GLOBAL: GlobalSettings = {
  moodleUrl: '',
  recentUrls: [],
  theme: 'system',
  language: 'auto',
  rememberLogin: false,
  background: false,
};

const GLOBAL_KEYS = ['moodleUrl', 'recentUrls', 'theme', 'language', 'rememberLogin', 'background'] as const;

export const DIR_PRESETS = [
  {id: 'sections', tpl: '{section}/{folder}'},
  {id: 'numbered', tpl: '{n} - {section}/{folder}'},
  {id: 'flat', tpl: ''},
];

export function splitPatch(patch: Partial<Settings>): {global: Partial<GlobalSettings>; site: Partial<SiteSettings>} {
  const global: Record<string, unknown> = {};
  const site: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(patch)) {
    ((GLOBAL_KEYS as readonly string[]).includes(k) ? global : site)[k] = v;
  }
  return {global: global as Partial<GlobalSettings>, site: site as Partial<SiteSettings>};
}

export function resolveSite(stored: Stored, host: string): SiteSettings {
  return {...DEFAULT_SITE, ...stored.profiles[''], ...(host ? stored.profiles[host] : {})};
}

const KEY = 'moodle-scraper-settings-v2';
const OLD_KEY = 'moodle-downloader-settings-v1';

/** Wandelt gespeicherte Daten beliebiger älterer Versionen in das aktuelle Schema um. */
export function migrateStored(raw: unknown): Stored {
  const o = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  const profiles =
    o.profiles && typeof o.profiles === 'object' ? (o.profiles as Stored['profiles']) : ({} as Stored['profiles']);
  return {
    ...DEFAULT_GLOBAL,
    ...(o as Partial<GlobalSettings>),
    version: SETTINGS_VERSION,
    profiles,
  };
}

/** Altes (v1) Format: flache Einstellungen -> Profil-Vorlage. */
export function migrateV1(raw: Record<string, unknown>): Stored {
  return migrateStored({
    moodleUrl: raw.moodleUrl,
    recentUrls: raw.recentUrls,
    theme: raw.theme,
    profiles: {'': {destDir: raw.destDir, zip: raw.zip, types: raw.types}},
  });
}

export function loadSettings(): Stored {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return migrateStored(JSON.parse(raw));
    const old = localStorage.getItem(OLD_KEY);
    if (old) return migrateV1(JSON.parse(old));
  } catch {
    /* ignore */
  }
  return migrateStored({});
}

export function saveSettings(s: Stored) {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    /* ignore */
  }
}

// Kursordner-Namen und Job-Aufbau

const SEMESTER_PREFIX = /^\s*[[(]?\s*(wise|ws|sose|ss|hs|fs)\s*[-_ ]?\d{2,4}([/-]\d{2,4})?\s*[\])]?\s*[-–:|]?\s*/i;
const YEAR_PREFIX = /^\s*20\d{2}([/-]\d{2,4})?\s*[-–:|]\s*/;
const SEMESTER_SUFFIX = /\s*[[(]\s*(wise|ws|sose|ss|hs|fs)[^\])]*[\])]\s*$/i;

export function cleanCourseName(name: string): string {
  const cleaned = name.replace(SEMESTER_PREFIX, '').replace(YEAR_PREFIX, '').replace(SEMESTER_SUFFIX, '').trim();
  return cleaned || name;
}

export function courseFolderName(c: {name: string; shortName: string}, mode: CourseNaming): string {
  if (mode === 'short') return c.shortName.trim() || cleanCourseName(c.name);
  if (mode === 'clean') return cleanCourseName(c.name);
  return c.name;
}

export interface BuildOptions {
  excluded: Set<string>;
  types: string[];
  maxSizeMb: number;
}

/** Filtert den Kursinhalt nach Dateityp, Größe und manuell abgewählten Dateien. */
export function buildCourseItems(content: CourseContent, o: BuildOptions): DownloadItem[] {
  const max = o.maxSizeMb > 0 ? o.maxSizeMb * 1024 * 1024 : null;
  const items: DownloadItem[] = [];
  for (const s of content.sections) {
    for (const it of s.items) {
      if (o.excluded.has(it.url) || !itemAllowed(it, o.types)) continue;
      if (max != null && it.size != null && it.size > max) continue;
      items.push({
        section: s.title,
        sectionIndex: s.number,
        folder: it.folder,
        filename: it.filename,
        url: it.url,
        size: it.size,
      });
    }
  }
  return items;
}

/** Alle externen Links eines Kurses (für Links.md). */
export function buildCourseLinks(content: CourseContent): JobLink[] {
  return content.sections.flatMap((s) => s.links.map((l) => ({section: s.title, name: l.name, url: l.url})));
}
