import { mount, type VueWrapper } from '@vue/test-utils'
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

const mounted: VueWrapper[] = []

function mountEditor(props: { article?: Article } = {}) {
  const wrapper = mount(Editor, { props })
  mounted.push(wrapper)
  return wrapper
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()))
afterEach(() => {
  for (const wrapper of mounted.splice(0)) wrapper.unmount()
  vi.unstubAllGlobals()
})

describe('Editor', () => {
  it('creates a draft via the proxy when saving a new article', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(201, draft))
    const wrapper = mountEditor()
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
    const wrapper = mountEditor({ article: draft })
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
    const wrapper = mountEditor({ article: draft })
    await wrapper.find('[data-test="save"]').trigger('click')
    await vi.waitFor(() => expect(wrapper.find('[data-test="error"]').exists()).toBe(true))
  })

  it('saves via Ctrl+S exactly like the save-draft button', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(200, draft))
    const wrapper = mountEditor({ article: draft })
    await wrapper.find('textarea').setValue('# 新内容')
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }))
    await vi.waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1)
    })
    const [url, init] = vi.mocked(fetch).mock.calls[0]
    expect(url).toBe('/api/admin/articles/7')
    expect(init?.method).toBe('PUT')
    expect(JSON.parse(String(init?.body))).toEqual({
      title: '已有草稿',
      slug: 'old-draft',
      markdown: '# 新内容',
    })
  })

  it('saves via Cmd+S on macOS', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(200, draft))
    mountEditor({ article: draft })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', metaKey: true }))
    await vi.waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1)
    })
  })

  it('prevents the browser default save dialog on Ctrl+S', () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(200, draft))
    mountEditor({ article: draft })
    const event = new KeyboardEvent('keydown', { key: 's', ctrlKey: true, cancelable: true })
    window.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(true)
  })

  it('ignores repeated Ctrl+S while a save is in flight', async () => {
    let resolveFetch: ((r: Response) => void) | undefined
    vi.mocked(fetch).mockImplementation(
      () =>
        new Promise<Response>((resolve) => {
          resolveFetch = resolve
        }),
    )
    mountEditor({ article: draft })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }))
    await vi.waitFor(() => {
      expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1)
    })
    resolveFetch?.(jsonResponse(200, draft))
  })

  it('shows a saved indicator after a successful save', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(200, draft))
    const wrapper = mountEditor({ article: draft })
    expect(wrapper.find('[data-test="saved"]').exists()).toBe(false)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }))
    await vi.waitFor(() => {
      expect(wrapper.find('[data-test="saved"]').exists()).toBe(true)
    })
  })
})
