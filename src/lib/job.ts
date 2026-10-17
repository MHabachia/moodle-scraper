import type {DownloadJob, DownloadSummary} from '../api';
import type {Settings} from '../utils';

export type JobCourses = DownloadJob['courses'];

/** Baut den Backend-Auftrag aus Einstellungen und den ausgewählten Kursen/Dateien. */
export function makeJob(s: Settings, courses: JobCourses): DownloadJob {
  return {
    destDir: s.destDir,
    zip: s.zip,
    dirTemplate: s.dirTemplate,
    incremental: s.incremental,
    parallel: s.parallel,
    maxBytes: s.maxSizeMb > 0 ? s.maxSizeMb * 1024 * 1024 : null,
    allowExecutables: s.types.includes('programs'),
    saveLinks: s.saveLinks,
    courses,
  };
}

/** Kurse mit den fehlgeschlagenen Dateien eines Ergebnisses (für "Wiederholen"). */
export function retryCourses(summary: DownloadSummary): JobCourses {
  return summary.results
    .map((r) => ({
      id: r.courseId,
      name: r.name,
      items: r.failed.flatMap((f) => (f.item ? [f.item] : [])),
      links: [],
    }))
    .filter((c) => c.items.length > 0);
}
