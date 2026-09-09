import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import LoginForm from './LoginForm.vue'

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()))
afterEach(() => vi.unstubAllGlobals())

describe('LoginForm', () => {
  it('shows a generic error on a wrong password (401)', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(401, { error: 'unauthorized' }))
    const wrapper = mount(LoginForm)
    await wrapper.find('input[type="password"]').setValue('wrong')
    await wrapper.find('form').trigger('submit')
    await vi.waitFor(() => {
      expect(wrapper.text()).toContain('密码错误，请重试。')
    })
  })

  it('shows the remaining lockout time when locked out (429)', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(429, { error: 'too_many_attempts', retry_after: 287 }),
    )
    const wrapper = mount(LoginForm)
    await wrapper.find('input[type="password"]').setValue('wrong')
    await wrapper.find('form').trigger('submit')
    await vi.waitFor(() => {
      expect(wrapper.text()).toContain('尝试次数过多，请 4 分 47 秒后重试。')
    })
  })

  it('shows seconds only when less than a minute remains', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(429, { error: 'too_many_attempts', retry_after: 42 }),
    )
    const wrapper = mount(LoginForm)
    await wrapper.find('input[type="password"]').setValue('wrong')
    await wrapper.find('form').trigger('submit')
    await vi.waitFor(() => {
      expect(wrapper.text()).toContain('尝试次数过多，请 42 秒后重试。')
    })
  })
})
