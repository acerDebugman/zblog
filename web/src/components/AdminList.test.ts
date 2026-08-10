import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AdminList from './AdminList.vue'
import type { Article } from '../lib/api'

function article(id: number, slug: string, title: string, status: 'draft' | 'published'): Article {
  return {
    id,
    title,
    slug,
    markdown: '',
    status,
    created_at: '2026-08-07 10:00:00',
    updated_at: '2026-08-09 11:00:00',
    published_at: status === 'published' ? '2026-08-08 09:00:00' : null,
  }
}

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

afterEach(() => vi.unstubAllGlobals())

describe('AdminList', () => {
  it('renders drafts with preview links and published with public links', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          json(200, [article(1, 'draft-one', '草稿甲', 'draft'), article(2, 'pub-one', '已发乙', 'published')]),
        ),
    )
    const wrapper = mount(AdminList)
    await flushPromises()
    const rows = wrapper.findAll('.post-item')
    expect(rows).toHaveLength(2)
    expect(rows[0].text()).toContain('草稿')
    expect(rows[0].find('a[href="/preview/draft-one"]').exists()).toBe(true)
    expect(rows[1].find('a[href="/posts/pub-one"]').exists()).toBe(true)
    expect(rows[0].find('a[href="/admin/edit/1"]').exists()).toBe(true)
    // negative: published row has no 草稿 badge
    expect(rows[1].text()).not.toContain('草稿')
  })

  it('shows the empty state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json(200, [])))
    const wrapper = mount(AdminList)
    await flushPromises()
    expect(wrapper.text()).toContain('还没有文章')
  })
})
