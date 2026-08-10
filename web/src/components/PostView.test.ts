import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import PostView from './PostView.vue'
import type { Article } from '../lib/api'

const article: Article = {
  id: 9,
  title: '数学笔记',
  slug: 'math-notes',
  markdown: '# 公式\n\n$e^{i\\pi}+1=0$',
  status: 'published',
  created_at: '2026-08-07 10:00:00',
  updated_at: '2026-08-07 10:00:00',
  published_at: '2026-08-08 09:00:00',
}

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => {
  window.history.pushState({}, '', '/posts/math-notes')
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('PostView', () => {
  it('renders the article with math and records the pageview', async () => {
    const calls: [unknown, unknown][] = []
    vi.stubGlobal(
      'fetch',
      vi.fn((url: unknown, init?: unknown) => {
        calls.push([url, init])
        const u = String(url)
        if (u === '/api/articles/math-notes') return Promise.resolve(json(200, article))
        if (u === '/api/pageviews') return Promise.resolve(new Response(null, { status: 201 }))
        return Promise.resolve(json(404, { error: 'not found' }))
      }),
    )
    const wrapper = mount(PostView)
    await flushPromises()
    await vi.waitFor(() => {
      expect(wrapper.html()).toContain('class="katex"')
    })
    expect(wrapper.text()).toContain('数学笔记')
    expect(wrapper.text()).toContain('发布于 2026-08-08')
    const pv = calls.find(([url]) => url === '/api/pageviews')
    expect(pv).toBeDefined()
    expect(JSON.parse(String((pv![1] as RequestInit).body))).toEqual({
      path: '/posts/math-notes',
      article_id: 9,
    })
  })

  it('shows a not-found message for an unknown slug and records nothing', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json(404, { error: 'not found' })))
    const wrapper = mount(PostView)
    await flushPromises()
    expect(wrapper.text()).toContain('文章不存在或尚未发布')
    expect(vi.mocked(fetch).mock.calls.map((c) => c[0])).not.toContain('/api/pageviews')
  })
})
