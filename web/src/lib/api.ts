import { z } from 'zod'

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

/** Redirect to the admin login page on a 401; returns whether it redirected. */
export function redirectOnUnauthorized(error: unknown): boolean {
  if (error instanceof ApiError && error.status === 401) {
    window.location.assign('/admin/login')
    return true
  }
  return false
}

async function request<T>(path: string, schema: z.ZodType<T>, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (init?.body && !headers.has('content-type')) headers.set('content-type', 'application/json')
  const res = await fetch(path, { ...init, headers })
  if (!res.ok) throw new ApiError(res.status, await res.text())
  const data: unknown = await res.json()
  return schema.parse(data)
}

async function requestVoid(path: string, init?: RequestInit): Promise<void> {
  const res = await fetch(path, init)
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

export async function login(password: string): Promise<void> {
  await request('/api/admin/login', z.object({ ok: z.boolean() }), jsonPost({ password }))
}

export async function logout(): Promise<void> {
  await request('/api/admin/logout', z.object({ ok: z.boolean() }), { method: 'POST' })
}

export async function me(): Promise<void> {
  await request('/api/admin/me', z.object({ ok: z.boolean() }))
}

export function adminListArticles(): Promise<Article[]> {
  return request('/api/admin/articles', z.array(articleSchema))
}

export function adminGetArticle(id: number): Promise<Article> {
  return request(`/api/admin/articles/${id}`, articleSchema)
}

export function adminGetArticleBySlug(slug: string): Promise<Article | null> {
  return nullOn404(request(`/api/admin/articles/by-slug/${slug}`, articleSchema))
}

export function adminCreateArticle(input: { title: string; markdown: string }): Promise<Article> {
  return request('/api/admin/articles', articleSchema, jsonPost(input))
}

export function adminUpdateArticle(
  id: number,
  input: { title: string; slug: string; markdown: string },
): Promise<Article> {
  return request(`/api/admin/articles/${id}`, articleSchema, jsonPut(input))
}

export function publishArticle(id: number): Promise<Article> {
  return request(`/api/admin/articles/${id}/publish`, articleSchema, { method: 'POST' })
}

export function unpublishArticle(id: number): Promise<Article> {
  return request(`/api/admin/articles/${id}/unpublish`, articleSchema, { method: 'POST' })
}

export function deleteArticle(id: number): Promise<void> {
  return requestVoid(`/api/admin/articles/${id}`, { method: 'DELETE' })
}

export function statsOverview(): Promise<StatsOverview> {
  return request('/api/admin/stats/overview', statsOverviewSchema)
}

export function statsDaily(days: number): Promise<DailyStat[]> {
  return request(`/api/admin/stats/daily?days=${days}`, z.array(dailyStatSchema))
}

export function statsArticles(): Promise<ArticleStat[]> {
  return request('/api/admin/stats/articles', z.array(articleStatSchema))
}

export function statsIps(limit: number): Promise<IpStat[]> {
  return request(`/api/admin/stats/ips?limit=${limit}`, z.array(ipStatSchema))
}

/** Record a pageview; analytics failures never break the page. */
export async function recordPageview(input: {
  path: string
  article_id: number | null
}): Promise<void> {
  try {
    await fetch('/api/pageviews', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    })
  } catch {
    // intentionally swallowed: stats must not take the site down
  }
}
