# zblog Single-Binary Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor zblog from two processes (Astro SSR + Rust API) into a single deployable Rust binary: Astro builds a static site, `rust-embed` embeds `web/dist` into the binary, axum serves it with SPA fallback next to the API, and every page renders client-side.

**Architecture:** Per ADR-0004 (supersedes ADR-0001/0003). Rust is the only process and the public entry. Astro is build-time-only; all data fetching happens in browser-side Vue islands against same-origin relative `/api/*` paths (no proxy, no cookie forwarding, no SSR). Pageview IP/UA/referer move to Rust (`ConnectInfo` + headers). The Markdown/KaTeX pipeline is unchanged and now runs exclusively in the browser (shared by PostView and the editor Live Preview).

**Tech Stack:** unchanged (Rust axum 0.8 + sqlx; Astro 7 static + Vue 3 + vitest). New deps: `rust-embed`, `mime_guess` (server). Removed: `@astrojs/node` (web).

## Global Constraints

- Pageview API contract change: `POST /api/pageviews` body is exactly `{ "path": string, "article_id": number|null }`. IP comes from `ConnectInfo<SocketAddr>`, UA/Referer from request headers. 400 on empty path only.
- SPA fallback rule (exact order): exact embedded file → `{path}/index.html` → longest ancestor `index.html` walk → embedded `404.html` with status 404. Unmatched `/api/*` paths return the JSON `AppError::NotFound` body, never HTML.
- All browser→API calls are same-origin relative paths; `lib/api.ts` loses the `base`/`cookie` opts entirely. Cookie travels automatically.
- Admin guard: pages are public shells; data fetches handle 401 via a new `redirectOnUnauthorized(error)` helper → `/admin/login`. The API's `require_auth` is the real boundary and does not change.
- Build order: `pnpm build` (web) before `cargo build` (server). A `server/build.rs` creates a placeholder `web/dist` when missing so the backend always compiles from a fresh clone.
- Rust: no unwrap/expect/panic outside tests; clippy clean under the existing strict lints; English strings. Frontend: TS strict, `<script setup>`, `type` over `interface`, zod on API payloads, UI copy Chinese with 中/英 spacing.
- Commits: Conventional Commits in Chinese. Work happens on a new branch `feat/single-binary` off master.
- Do not change: domain layer, repositories, admin/article/stats endpoints (except pageview ingestion), auth mechanism, markdown pipeline behavior, styles.

---

### Task 1: Backend — pageview ingestion via ConnectInfo + headers

**Files:**
- Modify: `server/src/interfaces/http/dto.rs` (shrink RecordPageviewRequest)
- Modify: `server/src/interfaces/http/pageviews.rs` (full rewrite, below)
- Modify: `server/src/main.rs` (connect_info)
- Test: `server/tests/api.rs` (harness + pageview test updates)

**Interfaces:**
- Produces: `POST /api/pageviews {path, article_id?}` → 201; 400 empty path. IP = peer address; UA/referer from `user-agent`/`referer` headers (empty string when absent).

- [ ] **Step 1: Update the failing test**

In `server/tests/api.rs`, `pageview_ingestion_and_stats` becomes (full replacement of that test):

```rust
#[tokio::test]
async fn pageview_ingestion_and_stats() {
    let app = spawn_app().await;
    let article = seed(&app.db_url, "Watched Post", "w", true).await;

    // negative: validation
    let res = app
        .client
        .post(format!("{}/api/pageviews", app.base_url))
        .json(&serde_json::json!({ "path": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    for _ in 0..3 {
        let res = app
            .client
            .post(format!("{}/api/pageviews", app.base_url))
            .header("user-agent", "curl-test")
            .json(&serde_json::json!({
                "path": "/posts/watched-post",
                "article_id": article.id
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // negative: stats are admin-only
    let res = app
        .client
        .get(format!("{}/api/admin/stats/overview", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    login(&app).await;
    let res = app
        .client
        .get(format!("{}/api/admin/stats/overview", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let overview: serde_json::Value = res.json().await.unwrap();
    assert_eq!(overview["total_pv"], 3);
    assert_eq!(overview["total_uv"], 1); // all from the loopback peer
    assert_eq!(overview["today_pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/daily?days=7", app.base_url))
        .send()
        .await
        .unwrap();
    let daily: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(daily.len(), 1);
    assert_eq!(daily[0]["pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/articles", app.base_url))
        .send()
        .await
        .unwrap();
    let by_article: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(by_article.len(), 1);
    assert_eq!(by_article[0]["title"], "Watched Post");
    assert_eq!(by_article[0]["pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/ips?limit=5", app.base_url))
        .send()
        .await
        .unwrap();
    let ips: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(ips.len(), 1);
    assert_eq!(ips[0]["ip"], "127.0.0.1");
    assert_eq!(ips[0]["count"], 3);
}
```

Also update the harness `spawn_app`: change the serve line to

```rust
    tokio::spawn(async move {
        axum::serve(listener, build_router(state).into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await
            .unwrap()
    });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml pageview`
Expected: FAIL — 400/415 mismatch (old DTO requires `ip`).

- [ ] **Step 3: Implement**

`server/src/interfaces/http/dto.rs` — replace the whole `RecordPageviewRequest` struct with:

```rust
/// Pageview ingestion payload (sent by the visitor's browser).
#[derive(Debug, Deserialize)]
pub struct RecordPageviewRequest {
    /// Request path, e.g. `/posts/hello`.
    pub path: String,
    /// Article id when the page is an article.
    pub article_id: Option<i64>,
}
```

`server/src/interfaces/http/pageviews.rs` — full replacement:

```rust
//! Pageview ingestion endpoint (public; called by the visitor's browser).

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::domain::pageview::NewPageview;
use crate::error::{AppError, Result};
use crate::interfaces::http::dto::RecordPageviewRequest;
use crate::interfaces::http::AppState;

/// `POST /api/pageviews` — record one pageview.
///
/// The IP is the peer address; user agent and referer come from headers.
///
/// # Errors
///
/// Returns `AppError::BadRequest` when `path` is empty, and `AppError::Db`
/// when the insert fails.
pub async fn record(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RecordPageviewRequest>,
) -> Result<StatusCode> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".to_owned()));
    }
    let header_value = |name: &str| -> String {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };
    state
        .pageviews
        .record(&NewPageview {
            path: body.path,
            article_id: body.article_id,
            ip: addr.ip().to_string(),
            user_agent: header_value("user-agent"),
            referer: header_value("referer"),
        })
        .await?;
    Ok(StatusCode::CREATED)
}
```

`server/src/main.rs` — change the serve call:

```rust
    axum::serve(
        listener,
        build_router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("server error: {e}")))?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: 15/15 PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "refactor(server): 访问上报改从连接与请求头采集访客信息"
```

---

### Task 2: Frontend — static config, simplified API client, delete SSR-only pieces

**Files:**
- Modify: `web/astro.config.mjs` (static + dev proxy)
- Modify: `web/package.json` (remove @astrojs/node via pnpm)
- Modify: `web/src/lib/api.ts` (full rewrite, below)
- Modify: `web/src/env.d.ts` (drop RUST_API_URL)
- Modify: `web/src/components/LoginForm.vue`, `Editor.vue`, `StatsDashboard.vue` (call-site updates only)
- Delete: `web/src/middleware.ts`, `web/src/pages/api/[...path].ts`, `web/.env`
- Test: `web/src/lib/api.test.ts` (full rewrite, below)

**Interfaces:**
- Produces (consumed by all later tasks): api.ts functions now take NO opts — `login(password)`, `logout()`, `me()`, `adminListArticles()`, `adminGetArticle(id)`, `adminGetArticleBySlug(slug)`, `adminCreateArticle(input)`, `adminUpdateArticle(id, input)`, `publishArticle(id)`, `unpublishArticle(id)`, `deleteArticle(id)`, `statsOverview()`, `statsDaily(days)`, `statsArticles()`, `statsIps(limit)`, `listPublished()`, `getPublishedBySlug(slug)`; new `redirectOnUnauthorized(error: unknown): boolean`; `recordPageview({ path, article_id })` (new payload shape, still never throws). Schemas unchanged.

- [ ] **Step 1: Rewrite the failing test**

`web/src/lib/api.test.ts` — full replacement:

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  adminCreateArticle,
  ApiError,
  deleteArticle,
  getPublishedBySlug,
  listPublished,
  login,
  redirectOnUnauthorized,
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
  it('listPublished uses the same-origin relative path and parses', async () => {
    vi.stubGlobal('fetch', mockFetch(200, [article]))
    const list = await listPublished()
    expect(list).toEqual([article])
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/articles')
  })

  it('adminCreateArticle posts json to the relative admin path', async () => {
    vi.stubGlobal('fetch', mockFetch(201, article))
    const created = await adminCreateArticle({ title: 'T', markdown: 'm' })
    expect(created.slug).toBe('hello-world')
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/admin/articles')
    expect(vi.mocked(fetch).mock.calls[0][1]?.method).toBe('POST')
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

  it('deleteArticle resolves on 204 without parsing a body', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })))
    await expect(deleteArticle(1)).resolves.toBeUndefined()
  })

  it('login failure surfaces as ApiError 401', async () => {
    vi.stubGlobal('fetch', mockFetch(401, { error: 'unauthorized' }))
    await expect(login('wrong')).rejects.toBeInstanceOf(ApiError)
    await expect(login('wrong')).rejects.toMatchObject({ status: 401 })
  })

  it('statsOverview hits the relative stats path', async () => {
    vi.stubGlobal('fetch', mockFetch(200, { total_pv: 1, total_uv: 1, today_pv: 0, today_uv: 0 }))
    await statsOverview()
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe('/api/admin/stats/overview')
  })
})

describe('redirectOnUnauthorized', () => {
  it('redirects on ApiError 401 and not on other errors', () => {
    const assign = vi.fn()
    vi.stubGlobal('window', { ...window, location: { ...window.location, assign } })
    expect(redirectOnUnauthorized(new ApiError(401, 'unauthorized'))).toBe(true)
    expect(assign).toHaveBeenCalledWith('/admin/login')
    expect(redirectOnUnauthorized(new ApiError(500, 'x'))).toBe(false)
    expect(redirectOnUnauthorized(new Error('x'))).toBe(false)
    vi.unstubAllGlobals()
  })
})
```

Note: if the `window` stub above fights jsdom (location is non-configurable in some jsdom versions), use `vi.spyOn(window.location, 'assign').mockImplementation(vi.fn())` inside the test instead and note the deviation.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/lib/api.test.ts`
Expected: FAIL — exports/signatures missing (opts removed, redirectOnUnauthorized missing).

- [ ] **Step 3: Implement**

`web/astro.config.mjs` — full replacement:

```js
import { defineConfig } from 'astro/config'
import vue from '@astrojs/vue'

export default defineConfig({
  integrations: [vue()],
  vite: {
    server: {
      proxy: { '/api': 'http://127.0.0.1:8080' },
    },
  },
})
```

```bash
cd web && pnpm remove @astrojs/node && rm -f .env && rm -f src/middleware.ts && rm -f 'src/pages/api/[...path].ts'
```

`web/src/env.d.ts` — full replacement:

```ts
/// <reference path="../.astro/types.d.ts" />
```

`web/src/lib/api.ts` — full replacement:

```ts
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
```

Call-site updates (mechanical, remove the opts/cookie arguments):

- `web/src/components/LoginForm.vue:13` — `await login(password.value)`
- `web/src/components/Editor.vue` — `adminCreateArticle({ title: title.value, markdown: markdown.value })`; `adminUpdateArticle(currentId.value, { ... })`; `publishArticle(currentId.value)`; `unpublishArticle(currentId.value)`; `deleteArticle(currentId.value)`
- `web/src/components/StatsDashboard.vue:23-26` — `statsOverview()`, `statsDaily(30)`, `statsArticles()`, `statsIps(20)`; also change the catch block to:
  ```ts
  } catch (e) {
    if (redirectOnUnauthorized(e)) return
    failed.value = true
  }
  ```
  (import `redirectOnUnauthorized` from '../lib/api')

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && pnpm vitest run`
Expected: the new api.test.ts suite passes; Editor/StatsDashboard tests updated in this same commit if they referenced removed opts (adjust expectations to the same URLs without opts — the URLs are unchanged). All suites green.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "refactor(web): 切换静态构建并简化 API 客户端为同源相对调用"
```

---

### Task 3: Frontend — article page goes client-side (PostView)

**Files:**
- Create: `web/src/components/PostView.vue`
- Create: `web/src/pages/posts/index.astro`
- Delete: `web/src/pages/posts/[slug].astro`
- Test: `web/src/components/PostView.test.ts`

**Interfaces:**
- Consumes: `getPublishedBySlug`, `recordPageview` (new shape), `renderMarkdown`.
- Produces: `/posts/{slug}` serves the shell; PostView parses the slug from `location.pathname`, renders the article, sets `document.title`, records the pageview with `article_id`.

- [ ] **Step 1: Write the failing test**

`web/src/components/PostView.test.ts`:

```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/components/PostView.test.ts`
Expected: FAIL — component missing.

- [ ] **Step 3: Implement**

`web/src/components/PostView.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getPublishedBySlug, recordPageview, type Article } from '../lib/api'
import { renderMarkdown } from '../lib/markdown'

const article = ref<Article | null>(null)
const missing = ref(false)
const html = ref('')

onMounted(async () => {
  const slug = location.pathname.replace(/^\/posts\/?/, '').replace(/\/+$/, '')
  const found = slug.length > 0 ? await getPublishedBySlug(slug) : null
  if (found === null) {
    missing.value = true
    return
  }
  article.value = found
  document.title = `${found.title} · zblog`
  html.value = await renderMarkdown(found.markdown)
  await recordPageview({ path: location.pathname, article_id: found.id })
})
</script>

<template>
  <p v-if="missing" class="muted">
    文章不存在或尚未发布。回 <a href="/">首页</a> 看看吧。
  </p>
  <article v-else-if="article" class="card">
    <h1 style="margin-top: 0">{{ article.title }}</h1>
    <p class="muted" style="font-size: 14px">发布于 {{ article.published_at?.slice(0, 10) }}</p>
    <div class="article-body" v-html="html" />
  </article>
</template>
```

`web/src/pages/posts/index.astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import PostView from '../../components/PostView.vue'
---

<Layout title="文章">
  <PostView client:load />
</Layout>
```

```bash
rm 'web/src/pages/posts/[slug].astro'
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run`
Expected: PostView tests PASS, all suites green.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "refactor(web): 文章页改为客户端渲染"
```

---

### Task 4: Frontend — home/about shells (HomeList + PageviewTracker)

**Files:**
- Create: `web/src/components/HomeList.vue`
- Create: `web/src/components/PageviewTracker.vue`
- Modify: `web/src/pages/index.astro`, `web/src/pages/about.astro`
- Test: `web/src/components/HomeList.test.ts`, `web/src/components/PageviewTracker.test.ts`

**Interfaces:**
- Consumes: `listPublished`, `groupByYear`, `formatDay`, `recordPageview`.
- Produces: `HomeList` (no props; fetches and renders year groups); `PageviewTracker` (no props; on mount records `{ path: location.pathname, article_id: null }`, renders nothing visible). Used with `client:load` in `index.astro` and `about.astro`.

- [ ] **Step 1: Write the failing tests**

`web/src/components/HomeList.test.ts`:

```ts
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import HomeList from './HomeList.vue'
import type { Article } from '../lib/api'

function article(id: number, slug: string, title: string, publishedAt: string): Article {
  return {
    id,
    title,
    slug,
    markdown: '',
    status: 'published',
    created_at: publishedAt,
    updated_at: publishedAt,
    published_at: publishedAt,
  }
}

function json(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

afterEach(() => vi.unstubAllGlobals())

describe('HomeList', () => {
  it('renders year groups newest first with formatted dates', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        json([
          article(1, 'a', '八月文章', '2026-08-07 10:00:00'),
          article(2, 'b', '三月文章', '2026-03-01 09:00:00'),
          article(3, 'c', '去年文章', '2025-12-31 08:00:00'),
        ]),
      ),
    )
    const wrapper = mount(HomeList)
    await flushPromises()
    const years = wrapper.findAll('.year-title').map((el) => el.text())
    expect(years).toEqual(['2026', '2025'])
    const items = wrapper.findAll('.post-item')
    expect(items).toHaveLength(3)
    expect(items[0].text()).toContain('08-07')
    expect(items[0].text()).toContain('八月文章')
    expect(items[0].find('a').attributes('href')).toBe('/posts/a')
    // negative: the 2025 article must not appear in the 2026 group
    const groups = wrapper.findAll('.year-group')
    expect(groups[0].text()).not.toContain('去年文章')
  })

  it('shows the empty state when there are no articles', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json([])))
    const wrapper = mount(HomeList)
    await flushPromises()
    expect(wrapper.text()).toContain('还没有文章')
  })
})
```

`web/src/components/PageviewTracker.test.ts`:

```ts
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && pnpm vitest run src/components/HomeList.test.ts src/components/PageviewTracker.test.ts`
Expected: FAIL — components missing.

- [ ] **Step 3: Implement**

`web/src/components/HomeList.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { listPublished, type Article } from '../lib/api'
import { formatDay, groupByYear } from '../lib/date'

type DatedArticle = Article & { published_at: string }

const groups = ref<{ year: string; items: DatedArticle[] }[]>([])
const loaded = ref(false)

onMounted(async () => {
  const articles = (await listPublished()).filter(
    (a): a is DatedArticle => a.published_at !== null,
  )
  groups.value = groupByYear(articles)
  loaded.value = true
})
</script>

<template>
  <p v-if="loaded && groups.length === 0" class="muted">还没有文章，敬请期待。</p>
  <section v-for="group in groups" :key="group.year" class="year-group">
    <h2 class="year-title">{{ group.year }}</h2>
    <div v-for="item in group.items" :key="item.slug" class="post-item">
      <span class="post-date">{{ formatDay(item.published_at) }}</span>
      <a :href="`/posts/${item.slug}`">{{ item.title }}</a>
    </div>
  </section>
</template>
```

`web/src/components/PageviewTracker.vue`:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { recordPageview } from '../lib/api'

onMounted(() => {
  void recordPageview({ path: location.pathname, article_id: null })
})
</script>

<template>
  <span style="display: none" aria-hidden="true" />
</template>
```

`web/src/pages/index.astro` — full replacement:

```astro
---
import Layout from '../layouts/Layout.astro'
import HomeList from '../components/HomeList.vue'
import PageviewTracker from '../components/PageviewTracker.vue'
---

<Layout title="首页">
  <section class="hero">
    <h1>你好，我是这里的主人 👋</h1>
    <p>
      欢迎来到我的小站。这里记录我的学习与生活。
      <a href="/about">了解更多 →</a>
    </p>
  </section>

  <HomeList client:load />
  <PageviewTracker client:load />
</Layout>
```

`web/src/pages/about.astro` — full replacement:

```astro
---
import Layout from '../layouts/Layout.astro'
import PageviewTracker from '../components/PageviewTracker.vue'
---

<Layout title="关于">
  <article class="card article-body">
    <h1>关于我</h1>
    <p>这里写一段自我介绍：你是谁，在做什么，为什么写这个博客。</p>
    <p>也可以放上你的联系方式、GitHub、兴趣爱好。随时回到 <a href="/">首页</a> 看看文章。</p>
  </article>
  <PageviewTracker client:load />
</Layout>
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && pnpm vitest run`
Expected: all suites green.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "refactor(web): 首页与关于页改为静态壳加客户端列表"
```

---

### Task 5: Frontend — admin and preview shells go client-side

**Files:**
- Create: `web/src/components/AdminList.vue`, `web/src/components/EditPage.vue`, `web/src/components/PreviewView.vue`
- Modify: `web/src/pages/admin/index.astro`, `web/src/pages/preview/index.astro` (new, replaces `[slug].astro`), `web/src/pages/admin/edit/index.astro` (new, replaces `[id].astro`)
- Delete: `web/src/pages/preview/[slug].astro`, `web/src/pages/admin/edit/[id].astro`
- Test: `web/src/components/AdminList.test.ts`, `web/src/components/PreviewView.test.ts`

**Interfaces:**
- Consumes: `adminListArticles`, `adminGetArticle`, `adminGetArticleBySlug`, `redirectOnUnauthorized`, `renderMarkdown`, `Editor` (props `{ article?: Article }`, unchanged).
- Produces: `/admin` shell + AdminList; `/admin/edit/{id}` → shell + EditPage (parses id, loads article, mounts Editor); `/preview/{slug}` → shell + PreviewView (banner + draft render, 401 → login).

- [ ] **Step 1: Write the failing tests**

`web/src/components/AdminList.test.ts`:

```ts
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AdminList from './AdminList.vue'
import type { Article } from '../lib/api'

function article(id: number, slug: string, title: string, status: 'draft' | 'published'): Article {
  return {
    id,
    title,
    slug,
    markdown: '',
    status,
    created_at: '2026-08-07 10:00:00',
    updated_at: '2026-08-09 11:00:00',
    published_at: status === 'published' ? '2026-08-08 09:00:00' : null,
  }
}

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

afterEach(() => vi.unstubAllGlobals())

describe('AdminList', () => {
  it('renders drafts with preview links and published with public links', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          json(200, [article(1, 'draft-one', '草稿甲', 'draft'), article(2, 'pub-one', '已发乙', 'published')]),
        ),
    )
    const wrapper = mount(AdminList)
    await flushPromises()
    const rows = wrapper.findAll('.post-item')
    expect(rows).toHaveLength(2)
    expect(rows[0].text()).toContain('草稿')
    expect(rows[0].find('a[href="/preview/draft-one"]').exists()).toBe(true)
    expect(rows[1].find('a[href="/posts/pub-one"]').exists()).toBe(true)
    expect(rows[0].find('a[href="/admin/edit/1"]').exists()).toBe(true)
    // negative: published row has no 草稿 badge
    expect(rows[1].text()).not.toContain('草稿')
  })

  it('shows the empty state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(json(200, [])))
    const wrapper = mount(AdminList)
    await flushPromises()
    expect(wrapper.text()).toContain('还没有文章')
  })
})
```

`web/src/components/PreviewView.test.ts`:

```ts
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && pnpm vitest run src/components/AdminList.test.ts src/components/PreviewView.test.ts`
Expected: FAIL — components missing.

- [ ] **Step 3: Implement**

`web/src/components/AdminList.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { adminListArticles, redirectOnUnauthorized, type Article } from '../lib/api'

const articles = ref<Article[]>([])
const loaded = ref(false)
const failed = ref(false)

onMounted(async () => {
  try {
    articles.value = await adminListArticles()
    loaded.value = true
  } catch (e) {
    if (redirectOnUnauthorized(e)) return
    failed.value = true
  }
})
</script>

<template>
  <p v-if="failed" class="error-text">加载失败，请稍后重试。</p>
  <div v-else class="card">
    <p v-if="loaded && articles.length === 0" class="muted">还没有文章，点击「新建文章」开始写作。</p>
    <div v-for="article in articles" :key="article.id" class="post-item">
      <span class="post-date">{{ article.updated_at.slice(0, 10) }}</span>
      <a :href="`/admin/edit/${article.id}`" style="flex: 1">{{ article.title }}</a>
      <template v-if="article.status === 'draft'">
        <span class="muted" style="font-size: 13px">草稿</span>
        <a :href="`/preview/${article.slug}`" style="font-size: 13px">预览</a>
      </template>
      <a v-else :href="`/posts/${article.slug}`" style="font-size: 13px">已发布</a>
    </div>
  </div>
</template>
```

`web/src/components/EditPage.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { adminGetArticle, redirectOnUnauthorized, type Article } from '../lib/api'
import Editor from './Editor.vue'

const article = ref<Article | null>(null)
const missing = ref(false)

onMounted(async () => {
  const id = Number(location.pathname.replace(/\/+$/, '').split('/').pop())
  if (!Number.isInteger(id) || id <= 0) {
    missing.value = true
    return
  }
  try {
    article.value = await adminGetArticle(id)
  } catch (e) {
    if (redirectOnUnauthorized(e)) return
    missing.value = true
  }
})
</script>

<template>
  <p v-if="missing" class="muted">文章不存在。回 <a href="/admin">文章管理</a>。</p>
  <Editor v-else-if="article" :article="article" />
</template>
```

`web/src/components/PreviewView.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { adminGetArticleBySlug, redirectOnUnauthorized, type Article } from '../lib/api'
import { renderMarkdown } from '../lib/markdown'

const article = ref<Article | null>(null)
const missing = ref(false)
const html = ref('')

onMounted(async () => {
  const slug = location.pathname.replace(/^\/preview\/?/, '').replace(/\/+$/, '')
  if (slug.length === 0) {
    missing.value = true
    return
  }
  try {
    const found = await adminGetArticleBySlug(slug)
    if (found === null) {
      missing.value = true
      return
    }
    article.value = found
    document.title = `预览 · ${found.title} · zblog`
    html.value = await renderMarkdown(found.markdown)
  } catch (e) {
    if (redirectOnUnauthorized(e)) return
    missing.value = true
  }
})
</script>

<template>
  <p v-if="missing" class="muted">文章不存在。回 <a href="/admin">文章管理</a>。</p>
  <template v-else-if="article">
    <div class="banner">草稿预览 — 此页面仅作者可见，发布前访客无法访问。</div>
    <article class="card">
      <h1 style="margin-top: 0">{{ article.title }}</h1>
      <p class="muted" style="font-size: 14px">
        状态：{{ article.status === 'draft' ? '草稿' : '已发布' }}
      </p>
      <div class="article-body" v-html="html" />
    </article>
  </template>
</template>
```

`web/src/pages/admin/index.astro` — full replacement:

```astro
---
import Layout from '../../layouts/Layout.astro'
import AdminList from '../../components/AdminList.vue'
---

<Layout title="文章管理">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
    <h1 style="font-size: 22px; margin: 0">文章管理</h1>
    <nav>
      <a class="btn btn-secondary" href="/admin/stats" style="margin-right: 10px">访问统计</a>
      <a class="btn" href="/admin/new">新建文章</a>
    </nav>
  </div>
  <AdminList client:load />
</Layout>
```

`web/src/pages/admin/edit/index.astro` (new, replaces `edit/[id].astro`):

```astro
---
import Layout from '../../../layouts/Layout.astro'
import EditPage from '../../../components/EditPage.vue'
---

<Layout title="编辑文章">
  <h1 style="font-size: 22px">编辑文章</h1>
  <EditPage client:load />
</Layout>
```

`web/src/pages/preview/index.astro` (new, replaces `preview/[slug].astro`):

```astro
---
import Layout from '../../layouts/Layout.astro'
import PreviewView from '../../components/PreviewView.vue'
---

<Layout title="预览">
  <PreviewView client:load />
</Layout>
```

```bash
rm 'web/src/pages/preview/[slug].astro' 'web/src/pages/admin/edit/[id].astro'
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && pnpm vitest run && pnpm astro check`
Expected: all suites green; astro check 0 errors.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "refactor(web): 后台与预览页改为客户端守卫的静态壳"
```

---

### Task 6: Backend — embedded static site with SPA fallback

**Files:**
- Modify: `server/Cargo.toml` (add rust-embed, mime_guess)
- Create: `server/build.rs`
- Create: `server/src/interfaces/http/site.rs`
- Create: `server/tests/fixtures/site/` (fixture html files for unit tests)
- Modify: `server/src/interfaces/http/mod.rs` (route + fallback)
- Test: `server/tests/api.rs` (append static-serving integration test)

**Interfaces:**
- Produces: `site::serve_site(uri: &Uri) -> Response` implementing the Global Constraints fallback rule; `GET /` serves `index.html`; router `.fallback(get(site::fallback))`; unknown `/api/*` → JSON `AppError::NotFound`.

- [ ] **Step 1: Write the failing test**

`server/src/interfaces/http/site.rs` starts as only the fixture-backed test module (fails to compile without the implementation):

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::serve_path_with;
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/site"]
    struct TestAssets;

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn serves_exact_asset_with_mime() {
        let response = serve_path_with::<TestAssets>("assets/app.js");
        assert_eq!(response.status(), 200);
        assert!(response.headers()["content-type"].to_str().unwrap().contains("javascript"));
        assert_eq!(body_string(response).await, "// app");
    }

    #[tokio::test]
    async fn directory_index_for_shell_routes() {
        assert_eq!(body_string(serve_path_with::<TestAssets>("about")).await, "<h1>about</h1>");
        assert_eq!(body_string(serve_path_with::<TestAssets>("")).await, "<h1>home</h1>");
    }

    #[tokio::test]
    async fn spa_falls_back_to_ancestor_index() {
        assert_eq!(
            body_string(serve_path_with::<TestAssets>("posts/hello-world")).await,
            "<h1>post shell</h1>"
        );
        assert_eq!(
            body_string(serve_path_with::<TestAssets>("admin/edit/3")).await,
            "<h1>edit shell</h1>"
        );
        assert_eq!(body_string(serve_path_with::<TestAssets>("admin")).await, "<h1>admin home</h1>");
    }

    #[tokio::test]
    async fn unknown_path_gets_404_page_with_404_status() {
        let response = serve_path_with::<TestAssets>("no-such-page");
        assert_eq!(response.status(), 404);
        assert_eq!(body_string(response).await, "<h1>not found</h1>");
    }

    #[tokio::test]
    async fn unknown_api_path_gets_json_404() {
        let response = serve_path_with::<TestAssets>("api/nope");
        assert_eq!(response.status(), 404);
        assert_eq!(body_string(response).await, "{\"error\":\"not found\"}");
    }
}
```

Fixture files under `server/tests/fixtures/site/`, each exactly one line as asserted:

- `index.html` → `<h1>home</h1>`
- `404.html` → `<h1>not found</h1>`
- `about/index.html` → `<h1>about</h1>`
- `posts/index.html` → `<h1>post shell</h1>`
- `admin/index.html` → `<h1>admin home</h1>`
- `admin/edit/index.html` → `<h1>edit shell</h1>`
- `assets/app.js` → `// app`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml site`
Expected: FAIL — module/methods undefined.

- [ ] **Step 3: Implement**

`server/Cargo.toml` — add to `[dependencies]`:

```toml
rust-embed = "8"
mime_guess = "2"
```

`server/build.rs`:

```rust
//! Ensure `web/dist` exists so the embedded site compiles from a fresh clone.

use std::path::Path;

const PLACEHOLDER_INDEX: &str = "<!doctype html><title>zblog</title><p>Run `pnpm build` in web/ first.</p>";
const PLACEHOLDER_404: &str = "<!doctype html><title>404</title><p>not found</p>";

fn main() {
    let dist = Path::new("../web/dist");
    if !dist.exists() {
        let result = std::fs::create_dir_all(dist).and_then(|()| {
            std::fs::write(dist.join("index.html"), PLACEHOLDER_INDEX)?;
            std::fs::write(dist.join("404.html"), PLACEHOLDER_404)
        });
        if let Err(e) = result {
            println!("cargo:warning=could not create placeholder web/dist: {e}");
        }
    }
    println!("cargo:rerun-if-changed=../web/dist");
}
```

`server/src/interfaces/http/site.rs` (above the test module):

```rust
//! Embedded static site serving with SPA fallback (see ADR-0004).

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;

/// The built Astro site, embedded into the binary (release) or read from
/// disk (debug builds).
#[derive(RustEmbed)]
#[folder = "../web/dist"]
pub struct SiteAssets;

/// `GET /` — the home page shell.
pub async fn index() -> Response {
    serve_path_with::<SiteAssets>("")
}

/// Fallback handler: static asset, or SPA shell, or 404.
pub async fn fallback(uri: Uri) -> Response {
    serve_path_with::<SiteAssets>(uri.path())
}

/// Resolve a request path to an embedded response.
///
/// Order: exact file → `{path}/index.html` → longest ancestor `index.html`
/// → `404.html` with status 404. Unmatched `/api/*` paths return the JSON
/// `AppError::NotFound` body instead of HTML.
#[must_use]
pub fn serve_path_with<T: RustEmbed>(path: &str) -> Response {
    let path = path.trim_matches('/');
    if path == "api" || path.starts_with("api/") {
        return crate::error::AppError::NotFound.into_response();
    }
    if let Some(response) = embedded::<T>(path) {
        return response;
    }
    if !path.is_empty() {
        if let Some(response) = embedded::<T>(&format!("{path}/index.html")) {
            return response;
        }
        let mut current = path;
        while let Some(pos) = current.rfind('/') {
            current = &current[..pos];
            if let Some(response) = embedded::<T>(&format!("{current}/index.html")) {
                return response;
            }
        }
    }
    not_found::<T>()
}

fn embedded<T: RustEmbed>(path: &str) -> Option<Response> {
    let asset = T::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(asset.data.into_owned()))
        .ok()
}

fn not_found<T: RustEmbed>() -> Response {
    match T::get("404.html") {
        Some(asset) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(asset.data.into_owned()))
            .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response()),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Router fragment for the static site (kept for `mod.rs` readability).
pub fn routes<S>(router: axum::Router<S>) -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.route("/", get(index)).fallback(get(fallback))
}
```

`server/src/interfaces/http/mod.rs` — add `pub mod site;` and change the final router line:

```rust
    let app = Router::new().merge(public).merge(admin);
    site::routes(app).with_state(state)
```

Append the integration test to `server/tests/api.rs`:

```rust
#[tokio::test]
async fn static_site_and_api_404_shapes() {
    let app = spawn_app().await;

    // home page shell (placeholder or real build) is served as HTML
    let res = app.client.get(format!("{}/", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let content_type = res.headers()["content-type"].to_str().unwrap().to_owned();
    assert!(content_type.contains("text/html"));

    // unknown API path → JSON 404, not HTML
    let res = app
        .client
        .get(format!("{}/api/definitely-not-a-route", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"], "not found");

    // unknown page path → 404 status
    let res = app
        .client
        .get(format!("{}/definitely/not/a/page", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: all PASS (7 integration + 14 unit, incl. 5 site tests), clippy clean. Note: with only the placeholder dist present, the integration test must still pass (it asserts shapes, not content).

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 嵌入静态站点并支持 SPA 回退"
```

---

### Task 7: Single-binary smoke + docs

**Files:**
- Modify: `README.md` (deploy/run sections rewritten for single binary)
- Modify: `AGENTS.md` (Directory Layout / Build & Test Commands updated)
- No production code changes.

- [ ] **Step 1: Full build and run**

```bash
cd web && pnpm build && cd ..
cd server && cargo build
HASH=$(cargo run --example hash_password -- 'dev-password' | tail -n1)
ZBLOG_PASSWORD_HASH="$HASH" ZBLOG_SESSION_SECRET="$(openssl rand -hex 64)" \
  ZBLOG_BIND_ADDR=127.0.0.1:8080 ./target/debug/zblog-server &
sleep 2
```

- [ ] **Step 2: Verify the single binary serves everything**

```bash
curl -s localhost:8080/ | grep -o '你好，我是这里的主人'                    # static shell
curl -s -o /dev/null -w '%{http_code}\n' localhost:8080/posts/anything    # 200 (SPA fallback)
curl -s -o /dev/null -w '%{http_code}\n' localhost:8080/admin             # 200 (shell; guard is client-side)
curl -s -o /dev/null -w '%{http_code}\n' localhost:8080/nope              # 404
curl -s localhost:8080/api/nope                                           # {"error":"not found"}
curl -s localhost:8080/api/health                                         # {"status":"ok"}

# full authoring flow against the single binary
curl -s -c /tmp/sb.jar -X POST localhost:8080/api/admin/login -H 'content-type: application/json' -d '{"password":"dev-password"}'
curl -s -b /tmp/sb.jar -X POST localhost:8080/api/admin/articles -H 'content-type: application/json' -d '{"title":"单二进制","markdown":"# 成了\n\n$E=mc^2$"}'
curl -s -b /tmp/sb.jar -X POST localhost:8080/api/admin/articles/1/publish
curl -s localhost:8080/api/articles | grep -o '单二进制'
curl -s -X POST localhost:8080/api/pageviews -H 'content-type: application/json' -H 'user-agent: smoke' -d '{"path":"/posts/post","article_id":1}'
curl -s -b /tmp/sb.jar localhost:8080/api/admin/stats/ips                 # 127.0.0.1 count 1

kill %1; rm -f server/zblog.db /tmp/sb.jar
```

- [ ] **Step 3: Prove release single-file deployment (optional but recommended)**

```bash
cd server && cargo build --release
mkdir -p /tmp/zblog-standalone && cp target/release/zblog-server /tmp/zblog-standalone/
cd /tmp/zblog-standalone
ZBLOG_PASSWORD_HASH="$HASH" ZBLOG_SESSION_SECRET="$(openssl rand -hex 64)" ./zblog-server &
sleep 1
curl -s localhost:8080/ | grep -o '你好，我是这里的主人'   # served from the embedded assets, no dist on disk here
kill %1 && rm -rf /tmp/zblog-standalone
```

- [ ] **Step 4: Update docs**

`README.md`: rewrite 部署/运行 sections — single binary story: build `cd web && pnpm build && cd ../server && cargo build --release`; run with `ZBLOG_PASSWORD_HASH` / `ZBLOG_SESSION_SECRET` / optional `ZBLOG_BIND_ADDR` (public-facing now — example `0.0.0.0:8080`) / `ZBLOG_DATABASE_URL`; dev flow: `cargo run` + `pnpm dev` (vite proxies `/api`). Remove every mention of the Node SSR process, `RUST_API_URL`, and the two-process topology; reference ADR-0004.

`AGENTS.md`: Directory Layout — note Astro is static/build-time-only, `server/build.rs` placeholder behavior, embedded `web/dist`; Build & Test Commands — add `cd web && pnpm build` before `cargo build --release`, keep test commands.

- [ ] **Step 5: Final sweep and commit**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets && cd web && pnpm vitest run && pnpm astro check`
Expected: everything green.

```bash
git add README.md AGENTS.md
git commit -m "docs: 更新为单二进制部署的运行手册"
```
