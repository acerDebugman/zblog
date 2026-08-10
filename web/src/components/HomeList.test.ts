import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import HomeList from './HomeList.vue'
import type { Article } from '../lib/api'

function article(id: number, slug: string, title: string, publishedAt: string): Article {
  return {
    id,
    title,
    slug,
    markdown: '',
    status: 'published',
    created_at: publishedAt,
    updated_at: publishedAt,
    published_at: publishedAt,
  }
}

function json(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

afterEach(() => vi.unstubAllGlobals())

describe('HomeList', () => {
  it('renders year groups newest first with formatted dates', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        json([
          article(1, 'a', '八月文章', '2026-08-07 10:00:00'),
          article(2, 'b', '三月文章', '2026-03-01 09:00:00'),
          article(3, 'c', '去年文章', '2025-12-31 08:00:00'),
        ]),
      ),
    )
    const wrapper = mount(HomeList)
    await flushPromises()
    const years = wrapper.findAll('.year-title').map((el) => el.text())
    expect(years).toEqual(['2026', '2025'])
    const items = wrapper.findAll('.post-item')
    expect(items).toHaveLength(3)
    expect(items[0].text()).toContain('08-07')
    expect(items[0].text()).toContain('八月文章')
    expect(items[0].find('a').attributes('href')).toBe('/posts/a')
    // negative: the 2025 article must not appear in the 2026 group
    const groups = wrapper.findAll('.year-group')
    expect(groups[0].text()).not.toContain('去年文章')
  })

  it('shows the empty state when there are no articles', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json([])))
    const wrapper = mount(HomeList)
    await flushPromises()
    expect(wrapper.text()).toContain('还没有文章')
  })
})
