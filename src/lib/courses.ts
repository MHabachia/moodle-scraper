import type {Course} from '../api';
import type {Filter, Sort} from '../types';

export const FILTERS: Filter[] = ['all', 'active', 'favourite', 'ended', 'hidden'];

export function countByFilter(courses: Course[]): Record<Filter, number> {
  const c: Record<Filter, number> = {all: 0, active: 0, favourite: 0, ended: 0, hidden: 0};
  for (const k of courses) {
    c.all++;
    if (!k.ended && !k.hidden) c.active++;
    if (k.favourite) c.favourite++;
    if (k.ended) c.ended++;
    if (k.hidden) c.hidden++;
  }
  return c;
}

function matchesFilter(k: Course, filter: Filter): boolean {
  switch (filter) {
    case 'active':
      return !k.ended && !k.hidden;
    case 'favourite':
      return k.favourite;
    case 'ended':
      return k.ended;
    case 'hidden':
      return k.hidden;
    default:
      return true;
  }
}

/** Filtert nach Status und Suchtext und sortiert (Name oder Fortschritt). */
export function visibleCourses(courses: Course[], search: string, filter: Filter, sort: Sort): Course[] {
  const q = search.trim().toLowerCase();
  const list = courses.filter(
    (k) => matchesFilter(k, filter) && (!q || `${k.name} ${k.shortName} ${k.category}`.toLowerCase().includes(q)),
  );
  return list.sort((a, b) =>
    sort === 'progress'
      ? (b.progress ?? -1) - (a.progress ?? -1)
      : a.name.localeCompare(b.name, 'de', {sensitivity: 'base'}),
  );
}
