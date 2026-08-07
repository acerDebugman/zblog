import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import StatsDashboard from './StatsDashboard.vue'

const overview = { total_pv: 10, total_uv: 4, today_pv: 3, today_uv: 2 }
const daily = [{ date: '2026-08-07', pv: 3, uv: 2 }]
const articles = [{ article_id: 1, title: '热文', slug: 'hot-post', pv: 7 }]
const ips = [{ ip: '1.2.3.4', count: 5, last_seen: '2026-08-07 12:00:00' }]

function json(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn((url: unknown) => {
      const u = String(url)
      if (u.includes('overview')) return Promise.resolve(json(overview))
      if (u.includes('daily')) return Promise.resolve(json(daily))
      if (u.includes('articles')) return Promise.resolve(json(articles))
      if (u.includes('ips')) return Promise.resolve(json(ips))
      return Promise.resolve(new Response('{}', { status: 404 }))
    }),
  )
})
afterEach(() => vi.unstubAllGlobals())

describe('StatsDashboard', () => {
  it('renders overview counters and all three tables', async () => {
    const wrapper = mount(StatsDashboard)
    await flushPromises()
    expect(wrapper.text()).toContain('总浏览量')
    expect(wrapper.text()).toContain('10')
    expect(wrapper.text()).toContain('热文')
    expect(wrapper.text()).toContain('1.2.3.4')
    expect(wrapper.text()).toContain('2026-08-07')
  })

  it('shows an error state when loading fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('x', { status: 500 })))
    const wrapper = mount(StatsDashboard)
    await flushPromises()
    expect(wrapper.text()).toContain('加载失败')
  })
})
