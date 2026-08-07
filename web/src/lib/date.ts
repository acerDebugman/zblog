/** Extract `MM-DD` from a SQLite UTC timestamp (`YYYY-MM-DD HH:MM:SS`). */
export function formatDay(utc: string): string {
  return utc.slice(5, 10)
}

/** Group items by the year of `published_at`; years descend, in-group order preserved. */
export function groupByYear<T extends { published_at: string }>(
  items: T[],
): { year: string; items: T[] }[] {
  const byYear = new Map<string, T[]>()
  for (const item of items) {
    const year = item.published_at.slice(0, 4)
    const group = byYear.get(year) ?? []
    group.push(item)
    byYear.set(year, group)
  }
  return [...byYear.entries()]
    .map(([year, groupItems]) => ({ year, items: groupItems }))
    .sort((a, b) => b.year.localeCompare(a.year))
}
