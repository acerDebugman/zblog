import { describe, expect, it } from 'vitest'
import { formatDay, groupByYear } from './date'

describe('formatDay', () => {
  it('extracts month-day from a SQLite UTC timestamp', () => {
    expect(formatDay('2026-08-07 12:34:56')).toBe('08-07')
    expect(formatDay('2025-01-30 00:00:01')).toBe('01-30')
  })
})

describe('groupByYear', () => {
  const items = [
    { slug: 'a', published_at: '2026-08-07 10:00:00' },
    { slug: 'b', published_at: '2026-03-01 09:00:00' },
    { slug: 'c', published_at: '2025-12-31 08:00:00' },
  ]

  it('groups by year with newest year first', () => {
    const groups = groupByYear(items)
    expect(groups.map((g) => g.year)).toEqual(['2026', '2025'])
  })

  it('keeps input order inside a group and never mixes years', () => {
    const groups = groupByYear(items)
    expect(groups[0].items.map((i) => i.slug)).toEqual(['a', 'b'])
    expect(groups[1].items.map((i) => i.slug)).toEqual(['c'])
    // negative: 2025 article must not leak into the 2026 group
    expect(groups[0].items.some((i) => i.slug === 'c')).toBe(false)
  })

  it('returns an empty list for empty input', () => {
    expect(groupByYear([])).toEqual([])
  })
})
