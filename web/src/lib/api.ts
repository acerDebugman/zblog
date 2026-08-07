import { z } from 'zod'

const API_BASE = import.meta.env.RUST_API_URL ?? 'http://127.0.0.1:8080'

export const articleSchema = z.object({
  id: z.number(),
  title: z.string(),
  slug: z.string(),
  markdown: z.string(),
  status: z.enum(['draft', 'published']),
  created_at: z.string(),
  updated_at: z.string(),
  published_at: z.string().nullable(),
})
export type Article = z.infer<typeof articleSchema>

export const statsOverviewSchema = z.object({
  total_pv: z.number(),
  total_uv: z.number(),
  today_pv: z.number(),
  today_uv: z.number(),
})
export type StatsOverview = z.infer<typeof statsOverviewSchema>

export const dailyStatSchema = z.object({ date: z.string(), pv: z.number(), uv: z.number() })
export type DailyStat = z.infer<typeof dailyStatSchema>

export const articleStatSchema = z.object({
  article_id: z.number(),
  title: z.string(),
  slug: z.string(),
  pv: z.number(),
})
export type ArticleStat = z.infer<typeof articleStatSchema>

export const ipStatSchema = z.object({
  ip: z.string(),
  count: z.number(),
  last_seen: z.string(),
})
export type IpStat = z.infer<typeof ipStatSchema>

export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

type Opts = { cookie?: string; base?: string }

async function request<T>(
  path: string,
  schema: z.ZodType<T>,
  init?: RequestInit,
  opts?: Opts,
): Promise<T> {
  const base = opts?.base ?? API_BASE
  const headers = new Headers(init?.headers)
  if (init?.body && !headers.has('content-type')) headers.set('content-type', 'application/json')
  if (opts?.cookie) headers.set('cookie', opts.cookie)
  const res = await fetch(`${base}${path}`, { ...init, headers })
  if (!res.ok) throw new ApiError(res.status, await res.text())
  const data: unknown = await res.json()
  return schema.parse(data)
}

async function requestVoid(path: string, init?: RequestInit, opts?: Opts): Promise<void> {
  const base = opts?.base ?? API_BASE
  const headers = new Headers(init?.headers)
  if (opts?.cookie) headers.set('cookie', opts.cookie)
  const res = await fetch(`${base}${path}`, { ...init, headers })
  if (!res.ok) throw new ApiError(res.status, await res.text())
}

async function nullOn404<T>(promise: Promise<T>): Promise<T | null> {
  try {
    return await promise
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) return null
    throw error
  }
}

const jsonPost = (body: unknown): RequestInit => ({ method: 'POST', body: JSON.stringify(body) })
const jsonPut = (body: unknown): RequestInit => ({ method: 'PUT', body: JSON.stringify(body) })

export function listPublished(): Promise<Article[]> {
  return request('/api/articles', z.array(articleSchema))
}

export function getPublishedBySlug(slug: string): Promise<Article | null> {
  return nullOn404(request(`/api/articles/${slug}`, articleSchema))
}

export async function login(password: string, opts?: Opts): Promise<void> {
  await request('/api/admin/login', z.object({ ok: z.boolean() }), jsonPost({ password }), opts)
}

export async function logout(opts?: Opts): Promise<void> {
  await request('/api/admin/logout', z.object({ ok: z.boolean() }), { method: 'POST' }, opts)
}

export async function me(cookie: string): Promise<void> {
  await request('/api/admin/me', z.object({ ok: z.boolean() }), undefined, { cookie })
}

export function adminListArticles(cookie: string): Promise<Article[]> {
  return request('/api/admin/articles', z.array(articleSchema), undefined, { cookie })
}

export function adminGetArticle(id: number, cookie: string): Promise<Article> {
  return request(`/api/admin/articles/${id}`, articleSchema, undefined, { cookie })
}

export function adminGetArticleBySlug(slug: string, cookie: string): Promise<Article | null> {
  return nullOn404(request(`/api/admin/articles/by-slug/${slug}`, articleSchema, undefined, { cookie }))
}

export function adminCreateArticle(
  input: { title: string; markdown: string },
  opts?: Opts,
): Promise<Article> {
  return request('/api/admin/articles', articleSchema, jsonPost(input), opts)
}

export function adminUpdateArticle(
  id: number,
  input: { title: string; slug: string; markdown: string },
  opts?: Opts,
): Promise<Article> {
  return request(`/api/admin/articles/${id}`, articleSchema, jsonPut(input), opts)
}

export function publishArticle(id: number, opts?: Opts): Promise<Article> {
  return request(`/api/admin/articles/${id}/publish`, articleSchema, { method: 'POST' }, opts)
}

export function unpublishArticle(id: number, opts?: Opts): Promise<Article> {
  return request(`/api/admin/articles/${id}/unpublish`, articleSchema, { method: 'POST' }, opts)
}

export function deleteArticle(id: number, opts?: Opts): Promise<void> {
  return requestVoid(`/api/admin/articles/${id}`, { method: 'DELETE' }, opts)
}

export function statsOverview(cookie: string, opts?: Opts): Promise<StatsOverview> {
  return request('/api/admin/stats/overview', statsOverviewSchema, undefined, { ...opts, cookie })
}

export function statsDaily(days: number, cookie: string, opts?: Opts): Promise<DailyStat[]> {
  return request(`/api/admin/stats/daily?days=${days}`, z.array(dailyStatSchema), undefined, { ...opts, cookie })
}

export function statsArticles(cookie: string, opts?: Opts): Promise<ArticleStat[]> {
  return request('/api/admin/stats/articles', z.array(articleStatSchema), undefined, { ...opts, cookie })
}

export function statsIps(limit: number, cookie: string, opts?: Opts): Promise<IpStat[]> {
  return request(`/api/admin/stats/ips?limit=${limit}`, z.array(ipStatSchema), undefined, { ...opts, cookie })
}

/** Record a pageview server-side; analytics failures never break page rendering. */
export async function recordPageview(input: {
  path: string
  article_id: number | null
  ip: string
  user_agent: string
  referer: string
}): Promise<void> {
  try {
    await fetch(`${API_BASE}/api/pageviews`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    })
  } catch {
    // intentionally swallowed: stats must not take the site down
  }
}
