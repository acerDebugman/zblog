import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  adminCreateArticle,
  ApiError,
  deleteArticle,
  getPublishedBySlug,
  listPublished,
  statsOverview,
  type Article,
} from './api'

const article: Article = {
  id: 1,
  title: 'Hello World',
  slug: 'hello-world',
  markdown: '# hi',
  status: 'published',
  created_at: '2026-08-07 10:00:00',
  updated_at: '2026-08-07 10:00:00',
  published_at: '2026-08-07 11:00:00',
}

function mockFetch(status: number, body: unknown) {
  return vi.fn().mockImplementation(() =>
    Promise.resolve(
      new Response(JSON.stringify(body), {
        status,
        headers: { 'content-type': 'application/json' },
      }),
    ),
  )
}

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn())
})
afterEach(() => {
  vi.unstubAllGlobals()
})

describe('api client', () => {
  it('listPublished hits the Rust base URL and parses', async () => {
    vi.stubGlobal('fetch', mockFetch(200, [article]))
    const list = await listPublished()
    expect(list).toEqual([article])
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('http://127.0.0.1:8080/api/articles')
  })

  it('browser callers use the same-origin proxy via base ""', async () => {
    vi.stubGlobal('fetch', mockFetch(201, article))
    const created = await adminCreateArticle({ title: 'T', markdown: 'm' }, { base: '' })
    expect(created.slug).toBe('hello-world')
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/admin/articles')
    expect(vi.mocked(fetch).mock.calls[0][1]?.method).toBe('POST')
  })

  it('browser stats calls use the same-origin proxy', async () => {
    vi.stubGlobal('fetch', mockFetch(200, { total_pv: 1, total_uv: 1, today_pv: 0, today_uv: 0 }))
    await statsOverview('', { base: '' })
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/admin/stats/overview')
  })

  it('getPublishedBySlug maps 404 to null', async () => {
    vi.stubGlobal('fetch', mockFetch(404, { error: 'not found' }))
    expect(await getPublishedBySlug('missing')).toBeNull()
  })

  it('throws ApiError with status on server errors', async () => {
    vi.stubGlobal('fetch', mockFetch(500, { error: 'boom' }))
    await expect(listPublished()).rejects.toMatchObject({ status: 500 })
  })

  it('rejects malformed payloads (zod)', async () => {
    vi.stubGlobal('fetch', mockFetch(200, [{ id: 1, title: 'x' }]))
    await expect(listPublished()).rejects.toThrow()
  })

  it('forwards the session cookie for admin SSR calls', async () => {
    vi.stubGlobal('fetch', mockFetch(200, { total_pv: 3, total_uv: 2, today_pv: 1, today_uv: 1 }))
    const overview = await statsOverview('zblog_auth=signed')
    expect(overview.total_pv).toBe(3)
    const headers = vi.mocked(fetch).mock.calls[0][1]?.headers as Headers
    expect(headers.get('cookie')).toBe('zblog_auth=signed')
  })

  it('deleteArticle resolves on 204 without parsing a body', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })))
    await expect(deleteArticle(1, { base: '' })).resolves.toBeUndefined()
  })

  it('login failure surfaces as ApiError 401', async () => {
    vi.stubGlobal('fetch', mockFetch(401, { error: 'unauthorized' }))
    const { login } = await import('./api')
    await expect(login('wrong', { base: '' })).rejects.toMatchObject({ status: 401 })
    await expect(login('wrong', { base: '' })).rejects.toBeInstanceOf(ApiError)
  })
})
