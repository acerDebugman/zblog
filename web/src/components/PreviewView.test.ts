import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import PreviewView from './PreviewView.vue'
import type { Article } from '../lib/api'

const draft: Article = {
  id: 5,
  title: '未发布稿件',
  slug: 'wip',
  markdown: '# 半成品',
  status: 'draft',
  created_at: '2026-08-07 10:00:00',
  updated_at: '2026-08-07 10:00:00',
  published_at: null,
}

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => {
  window.history.pushState({}, '', '/preview/wip')
})
afterEach(() => vi.unstubAllGlobals())

describe('PreviewView', () => {
  it('renders the draft with the banner and status', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json(200, draft)))
    const wrapper = mount(PreviewView)
    await flushPromises()
    await vi.waitFor(() => expect(wrapper.html()).toContain('<h1'))
    expect(wrapper.text()).toContain('草稿预览')
    expect(wrapper.text()).toContain('状态：草稿')
    expect(wrapper.text()).toContain('未发布稿件')
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/admin/articles/by-slug/wip')
  })

  it('shows a missing message for an unknown slug', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json(404, { error: 'not found' })))
    const wrapper = mount(PreviewView)
    await flushPromises()
    expect(wrapper.text()).toContain('文章不存在')
  })
})
