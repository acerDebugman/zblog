import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Editor from './Editor.vue'
import type { Article } from '../lib/api'

const draft: Article = {
  id: 7,
  title: '已有草稿',
  slug: 'old-draft',
  markdown: '# 旧内容',
  status: 'draft',
  created_at: '2026-08-07 10:00:00',
  updated_at: '2026-08-07 10:00:00',
  published_at: null,
}

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()))
afterEach(() => vi.unstubAllGlobals())

describe('Editor', () => {
  it('creates a draft via the proxy when saving a new article', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(201, draft))
    const wrapper = mount(Editor, { props: {} })
    await wrapper.find('input[data-test="title"]').setValue('新文章')
    await wrapper.find('textarea').setValue('# 正文')
    await wrapper.find('[data-test="save"]').trigger('click')
    await vi.waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledWith(
        '/api/admin/articles',
        expect.objectContaining({ method: 'POST' }),
      )
    })
    const body = JSON.parse(String(vi.mocked(fetch).mock.calls[0][1]?.body))
    expect(body).toEqual({ title: '新文章', markdown: '# 正文' })
  })

  it('updates then publishes an existing draft', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(jsonResponse(200, draft)) // update
      .mockResolvedValueOnce(jsonResponse(200, { ...draft, status: 'published' })) // publish
    const wrapper = mount(Editor, { props: { article: draft } })
    await wrapper.find('[data-test="publish"]').trigger('click')
    await vi.waitFor(() => {
      expect(vi.mocked(fetch).mock.calls.map((c) => c[0])).toEqual([
        '/api/admin/articles/7',
        '/api/admin/articles/7/publish',
      ])
    })
    expect(wrapper.text()).toContain('已发布')
  })

  it('shows an error when the server rejects the save', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(400, { error: 'slug already exists' }))
    const wrapper = mount(Editor, { props: { article: draft } })
    await wrapper.find('[data-test="save"]').trigger('click')
    await vi.waitFor(() => expect(wrapper.find('[data-test="error"]').exists()).toBe(true))
  })
})
