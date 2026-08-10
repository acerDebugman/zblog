import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import PageviewTracker from './PageviewTracker.vue'

afterEach(() => vi.unstubAllGlobals())

describe('PageviewTracker', () => {
  it('posts a pageview for the current path on mount', async () => {
    window.history.pushState({}, '', '/about')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(null, { status: 201 })),
    )
    mount(PageviewTracker)
    await flushPromises()
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/pageviews')
    const body = JSON.parse(String(vi.mocked(fetch).mock.calls[0][1]?.body))
    expect(body).toEqual({ path: '/about', article_id: null })
  })
})
