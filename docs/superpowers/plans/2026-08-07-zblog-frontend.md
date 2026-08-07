# zblog Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Astro SSR + Vue frontend for zblog: bright/cheerful public pages (home with year-grouped article list, about, article with math rendering), draft preview, and an admin backend (login, article list, markdown editor with live preview, traffic stats dashboard).

**Architecture:** Astro in SSR mode (Node adapter, standalone) is the only public entry (ADR-0001/0003). SSR frontmatter calls the Rust API directly at `RUST_API_URL` (default `http://127.0.0.1:8080`); browser-side Vue islands call a thin same-origin proxy endpoint `/api/*` which forwards to Rust (including cookies). One shared module `lib/markdown.ts` renders Markdown+math (remark/rehype + KaTeX) — used by SSR pages and the editor's Live Preview alike. Style is hand-rolled CSS with warm bright design tokens (简洁、明亮、快乐); no CSS framework.

**Tech Stack:** Astro 5, @astrojs/node 9, @astrojs/vue 5, Vue 3 (`<script setup>`), TypeScript strict, zod, unified/remark/rehype/KaTeX, vitest + @vue/test-utils + jsdom.

## Global Constraints

- The Rust backend (`docs/superpowers/plans/2026-08-07-zblog-backend.md`) must be implemented first — this plan consumes its exact API.
- TypeScript strict mode; `type` over `interface`; Props/Emits explicitly typed; `<script setup>` only; validate all API payloads with zod (AGENTS.md).
- UI copy is Chinese; 中文与英文/数字之间留一个空格（如「共 42 篇文章」）.
- Timestamps from the API are UTC `YYYY-MM-DD HH:MM:SS`; display helpers slice, never `new Date()`.
- No analytics in preview routes; pageviews recorded only for public pages.
- Commits follow Conventional Commits in Chinese, e.g. `feat(web): 添加首页文章列表`.
- Domain terms per `CONTEXT.md`: Article / Draft / Published / Slug / PageView / Live Preview / Draft Preview.

---

### Task 1: Astro scaffold, layout, global styles

**Files:**
- Create: `web/` scaffold via `pnpm create astro`
- Create: `web/astro.config.mjs`, `web/src/env.d.ts`
- Create: `web/src/styles/global.css`
- Create: `web/src/layouts/Layout.astro`
- Create: `web/src/pages/index.astro` (placeholder, replaced in Task 5)
- Modify: `web/package.json` (test script)

**Interfaces:**
- Produces: `Layout.astro` with props `{ title: string }`, site header nav（首页 / 关于）, slot, footer; imports KaTeX CSS and `global.css`. Design tokens `--bg/--surface/--text/--muted/--accent/--accent-soft/--radius`.

- [ ] **Step 1: Scaffold the project**

```bash
cd /home/algo/pyspace/zgc/zblog
pnpm create astro@latest web -- --template minimal --install --no-git --typescript strict --skip-houston
cd web
pnpm astro add vue node --yes
pnpm add unified remark-parse remark-gfm remark-math remark-rehype rehype-katex rehype-stringify katex zod
pnpm add -D vitest @vue/test-utils jsdom @vitejs/plugin-vue
printf 'RUST_API_URL=http://127.0.0.1:8080\n' > .env
```

`web/astro.config.mjs` (verify it matches; `astro add` writes most of it):

```js
import { defineConfig } from 'astro/config'
import vue from '@astrojs/vue'
import node from '@astrojs/node'

export default defineConfig({
  output: 'server',
  adapter: node({ mode: 'standalone' }),
  integrations: [vue()],
})
```

`web/src/env.d.ts` (replace generated content):

```ts
/// <reference path="../.astro/types.d.ts" />

interface ImportMetaEnv {
  readonly RUST_API_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
```

Add to `web/package.json` scripts: `"test": "vitest run"`.

- [ ] **Step 2: Write global styles and layout**

`web/src/styles/global.css`:

```css
:root {
  --bg: #fffdf5;
  --surface: #ffffff;
  --text: #2b2b33;
  --muted: #8a8a93;
  --accent: #f59e0b;
  --accent-soft: #fef3c7;
  --radius: 14px;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: ui-sans-serif, system-ui, -apple-system, 'PingFang SC', 'Hiragino Sans GB',
    'Microsoft YaHei', sans-serif;
  line-height: 1.75;
}

a { color: inherit; text-decoration: none; }
a:hover { color: var(--accent); }

.site-shell { max-width: 760px; margin: 0 auto; padding: 0 20px; }

.site-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 28px 0 20px;
}
.site-title { font-size: 22px; font-weight: 700; letter-spacing: 0.5px; }
.site-nav a { margin-left: 20px; color: var(--muted); font-size: 15px; }
.site-nav a:hover { color: var(--accent); }

.card {
  background: var(--surface);
  border-radius: var(--radius);
  padding: 24px 28px;
  box-shadow: 0 2px 12px rgb(245 158 11 / 8%);
}

.hero {
  background: linear-gradient(120deg, var(--accent-soft), #fff 70%);
  border-radius: var(--radius);
  padding: 32px 28px;
  margin-bottom: 32px;
}
.hero h1 { margin: 0 0 8px; font-size: 26px; }
.hero p { margin: 0; color: var(--muted); }

.year-group { margin-bottom: 28px; }
.year-title { font-size: 20px; font-weight: 700; margin: 0 0 8px; color: var(--accent); }
.post-item {
  display: flex;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px dashed #f0e6d2;
}
.post-date { color: var(--muted); font-variant-numeric: tabular-nums; flex: none; }

.article-body { font-size: 16px; }
.article-body h1, .article-body h2, .article-body h3 { line-height: 1.3; }
.article-body pre {
  background: #fff8e6;
  padding: 14px 16px;
  border-radius: 10px;
  overflow-x: auto;
}
.article-body code { font-family: ui-monospace, 'SF Mono', Menlo, monospace; font-size: 0.92em; }
.article-body blockquote {
  margin: 0;
  padding: 4px 16px;
  border-left: 4px solid var(--accent);
  background: var(--accent-soft);
  border-radius: 0 10px 10px 0;
}
.article-body table { border-collapse: collapse; }
.article-body th, .article-body td { border: 1px solid #eadfc8; padding: 6px 12px; }

.banner {
  background: var(--accent-soft);
  border: 1px solid var(--accent);
  border-radius: 10px;
  padding: 10px 16px;
  margin-bottom: 20px;
  font-size: 14px;
}

.btn {
  display: inline-block;
  border: none;
  border-radius: 10px;
  padding: 8px 18px;
  background: var(--accent);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}
.btn:hover { opacity: 0.9; color: #fff; }
.btn-secondary { background: var(--accent-soft); color: var(--text); }
.btn-danger { background: #ef4444; }

.input {
  width: 100%;
  border: 1px solid #eadfc8;
  border-radius: 10px;
  padding: 10px 14px;
  font-size: 15px;
  background: var(--surface);
  color: var(--text);
}
.input:focus { outline: 2px solid var(--accent-soft); border-color: var(--accent); }

.muted { color: var(--muted); }
.error-text { color: #ef4444; font-size: 14px; }
.site-footer { padding: 40px 0 28px; color: var(--muted); font-size: 13px; text-align: center; }
```

`web/src/layouts/Layout.astro`:

```astro
---
import 'katex/dist/katex.min.css'
import '../styles/global.css'

type Props = { title: string }
const { title } = Astro.props
---

<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title} · zblog</title>
  </head>
  <body>
    <div class="site-shell">
      <header class="site-header">
        <a class="site-title" href="/">zblog</a>
        <nav class="site-nav">
          <a href="/">首页</a>
          <a href="/about">关于</a>
        </nav>
      </header>
      <main>
        <slot />
      </main>
      <footer class="site-footer">用 ❤ 与 Rust 构建 · {new Date().getFullYear()}</footer>
    </div>
  </body>
</html>
```

`web/src/pages/index.astro` (placeholder for now):

```astro
---
import Layout from '../layouts/Layout.astro'
---

<Layout title="首页">
  <div class="hero">
    <h1>你好，这里是我的博客</h1>
    <p>文章列表即将到来。</p>
  </div>
</Layout>
```

- [ ] **Step 3: Verify dev server renders**

Run: `cd web && pnpm dev` (background), then `curl -s localhost:4321/ | grep -o '你好，这里是我的博客'`
Expected: the greeting string. Stop the dev server.

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): 初始化 Astro SSR 工程与全局样式"
```

---

### Task 2: Markdown + math rendering pipeline

**Files:**
- Create: `web/src/lib/markdown.ts`
- Test: `web/src/lib/markdown.test.ts`
- Create: `web/vitest.config.ts`

**Interfaces:**
- Produces: `renderMarkdown(source: string): Promise<string>` — GFM + `$…$`/`$$…$$` math → HTML string with KaTeX markup. Shared by SSR pages and the editor Live Preview.

- [ ] **Step 1: Write the failing test**

`web/src/lib/markdown.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { renderMarkdown } from './markdown'

describe('renderMarkdown', () => {
  it('renders headings and emphasis', async () => {
    const html = await renderMarkdown('# 你好 *世界*')
    expect(html).toContain('<h1>')
    expect(html).toContain('<em>世界</em>')
  })

  it('renders GFM tables', async () => {
    const html = await renderMarkdown('| a | b |\n| - | - |\n| 1 | 2 |')
    expect(html).toContain('<table>')
    expect(html).toContain('<td>2</td>')
  })

  it('renders inline math through KaTeX', async () => {
    const html = await renderMarkdown('能量公式 $E=mc^2$ 很常见')
    expect(html).toContain('class="katex"')
  })

  it('renders display math as katex-display', async () => {
    const html = await renderMarkdown('$$\\int_0^1 x\\,dx$$')
    expect(html).toContain('katex-display')
  })

  it('does not treat plain dollar amounts as math', async () => {
    const html = await renderMarkdown('价格是 5 美元，不是 $ 符号')
    expect(html).not.toContain('class="katex"')
  })
})
```

`web/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: { environment: 'jsdom' },
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/lib/markdown.test.ts`
Expected: FAIL — cannot find module `./markdown`.

- [ ] **Step 3: Write minimal implementation**

`web/src/lib/markdown.ts`:

```ts
import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import remarkRehype from 'remark-rehype'
import rehypeKatex from 'rehype-katex'
import rehypeStringify from 'rehype-stringify'

const pipeline = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkMath)
  .use(remarkRehype)
  .use(rehypeKatex)
  .use(rehypeStringify)

/** Render Markdown source (GFM + $...$/$$...$$ math) to an HTML string. */
export async function renderMarkdown(source: string): Promise<string> {
  const file = await pipeline.process(source)
  return String(file)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run src/lib/markdown.test.ts`
Expected: 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): 添加 Markdown 与数学公式渲染管线"
```

---

### Task 3: Date utilities (year grouping, day formatting)

**Files:**
- Create: `web/src/lib/date.ts`
- Test: `web/src/lib/date.test.ts`

**Interfaces:**
- Produces:
  - `formatDay(utc: string): string` — `'2026-08-07 12:00:00'` → `'08-07'`
  - `groupByYear<T extends { published_at: string }>(items: T[]): { year: string; items: T[] }[]` — groups by `published_at.slice(0, 4)`, preserves input order inside groups, groups sorted by year desc.

- [ ] **Step 1: Write the failing test**

`web/src/lib/date.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { formatDay, groupByYear } from './date'

describe('formatDay', () => {
  it('extracts month-day from a SQLite UTC timestamp', () => {
    expect(formatDay('2026-08-07 12:34:56')).toBe('08-07')
    expect(formatDay('2025-01-30 00:00:01')).toBe('01-30')
  })
})

describe('groupByYear', () => {
  const items = [
    { slug: 'a', published_at: '2026-08-07 10:00:00' },
    { slug: 'b', published_at: '2026-03-01 09:00:00' },
    { slug: 'c', published_at: '2025-12-31 08:00:00' },
  ]

  it('groups by year with newest year first', () => {
    const groups = groupByYear(items)
    expect(groups.map((g) => g.year)).toEqual(['2026', '2025'])
  })

  it('keeps input order inside a group and never mixes years', () => {
    const groups = groupByYear(items)
    expect(groups[0].items.map((i) => i.slug)).toEqual(['a', 'b'])
    expect(groups[1].items.map((i) => i.slug)).toEqual(['c'])
    // negative: 2025 article must not leak into the 2026 group
    expect(groups[0].items.some((i) => i.slug === 'c')).toBe(false)
  })

  it('returns an empty list for empty input', () => {
    expect(groupByYear([])).toEqual([])
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/lib/date.test.ts`
Expected: FAIL — module missing.

- [ ] **Step 3: Write minimal implementation**

`web/src/lib/date.ts`:

```ts
/** Extract `MM-DD` from a SQLite UTC timestamp (`YYYY-MM-DD HH:MM:SS`). */
export function formatDay(utc: string): string {
  return utc.slice(5, 10)
}

/** Group items by the year of `published_at`; years descend, in-group order preserved. */
export function groupByYear<T extends { published_at: string }>(
  items: T[],
): { year: string; items: T[] }[] {
  const byYear = new Map<string, T[]>()
  for (const item of items) {
    const year = item.published_at.slice(0, 4)
    const group = byYear.get(year) ?? []
    group.push(item)
    byYear.set(year, group)
  }
  return [...byYear.entries()]
    .map(([year, groupItems]) => ({ year, items: groupItems }))
    .sort((a, b) => b.year.localeCompare(a.year))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run src/lib/date.test.ts`
Expected: 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): 添加日期工具与按年分组"
```

---

### Task 4: Typed API client, `/api/*` proxy, auth middleware

**Files:**
- Create: `web/src/lib/api.ts`
- Test: `web/src/lib/api.test.ts`
- Create: `web/src/pages/api/[...path].ts`
- Create: `web/src/middleware.ts`

**Interfaces:**
- Consumes: backend API from `2026-08-07-zblog-backend.md` Tasks 6–9.
- Produces (all in `lib/api.ts`; `opts?: { cookie?: string; base?: string }` — SSR passes `cookie` from the incoming request; browser callers pass `{ base: '' }` to hit the same-origin proxy):
  - `articleSchema` / `type Article` — `{ id: number; title: string; slug: string; markdown: string; status: 'draft' | 'published'; created_at: string; updated_at: string; published_at: string | null }`
  - `class ApiError extends Error { readonly status: number }`
  - `listPublished(): Promise<Article[]>` → `GET /api/articles`
  - `getPublishedBySlug(slug): Promise<Article | null>` → `GET /api/articles/{slug}` (404 → `null`)
  - `login(password, opts?): Promise<void>` → `POST /api/admin/login`
  - `logout(opts?): Promise<void>` → `POST /api/admin/logout`
  - `me(cookie): Promise<void>` → `GET /api/admin/me` (throws `ApiError` 401)
  - `adminListArticles(cookie): Promise<Article[]>` → `GET /api/admin/articles`
  - `adminGetArticle(id, cookie): Promise<Article>` → `GET /api/admin/articles/{id}`
  - `adminGetArticleBySlug(slug, cookie): Promise<Article | null>` → `GET /api/admin/articles/by-slug/{slug}` (404 → `null`)
  - `adminCreateArticle(input: { title: string; markdown: string }, opts?): Promise<Article>` → `POST /api/admin/articles` (201)
  - `adminUpdateArticle(id, input: { title: string; slug: string; markdown: string }, opts?): Promise<Article>` → `PUT /api/admin/articles/{id}`
  - `publishArticle(id, opts?): Promise<Article>` → `POST /api/admin/articles/{id}/publish`
  - `unpublishArticle(id, opts?): Promise<Article>` → `POST /api/admin/articles/{id}/unpublish`
  - `deleteArticle(id, opts?): Promise<void>` → `DELETE /api/admin/articles/{id}` (204)
  - `statsOverview(cookie): Promise<StatsOverview>` — `{ total_pv, total_uv, today_pv, today_uv }`
  - `statsDaily(days, cookie): Promise<DailyStat[]>` — `{ date, pv, uv }`
  - `statsArticles(cookie): Promise<ArticleStat[]>` — `{ article_id, title, slug, pv }`
  - `statsIps(limit, cookie): Promise<IpStat[]>` — `{ ip, count, last_seen }`
  - `recordPageview(input: { path: string; article_id: number | null; ip: string; user_agent: string; referer: string }): Promise<void>` — server-side only, never throws.
- Produces (infra):
  - `web/src/pages/api/[...path].ts` — `ALL` endpoint proxying every method to `${RUST_API_URL}/api/{path}{search}`, forwarding request headers (minus host/connection/content-length) and body, and appending all `set-cookie` response headers.
  - `web/src/middleware.ts` — redirects unauthenticated requests to `/admin/login` for `/admin/**` (except `/admin/login`) and `/preview/**`.

- [ ] **Step 1: Write the failing test**

`web/src/lib/api.test.ts`:

```ts
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
  return vi.fn().mockResolvedValue(
    new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json' },
    }),
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/lib/api.test.ts`
Expected: FAIL — module missing.

- [ ] **Step 3: Write minimal implementation**

`web/src/lib/api.ts`:

```ts
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

export function statsOverview(cookie: string): Promise<StatsOverview> {
  return request('/api/admin/stats/overview', statsOverviewSchema, undefined, { cookie })
}

export function statsDaily(days: number, cookie: string): Promise<DailyStat[]> {
  return request(`/api/admin/stats/daily?days=${days}`, z.array(dailyStatSchema), undefined, { cookie })
}

export function statsArticles(cookie: string): Promise<ArticleStat[]> {
  return request('/api/admin/stats/articles', z.array(articleStatSchema), undefined, { cookie })
}

export function statsIps(limit: number, cookie: string): Promise<IpStat[]> {
  return request(`/api/admin/stats/ips?limit=${limit}`, z.array(ipStatSchema), undefined, { cookie })
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
```

`web/src/pages/api/[...path].ts`:

```ts
import type { APIRoute } from 'astro'

const API_BASE = import.meta.env.RUST_API_URL ?? 'http://127.0.0.1:8080'
const STRIP_REQUEST = new Set(['host', 'connection', 'content-length'])

export const ALL: APIRoute = async ({ params, request }) => {
  const url = new URL(request.url)
  const target = `${API_BASE}/api/${params.path ?? ''}${url.search}`

  const headers = new Headers()
  for (const [key, value] of request.headers) {
    if (!STRIP_REQUEST.has(key.toLowerCase())) headers.set(key, value)
  }
  const body =
    request.method === 'GET' || request.method === 'HEAD' ? undefined : await request.arrayBuffer()

  const upstream = await fetch(target, { method: request.method, headers, body })

  const responseHeaders = new Headers()
  for (const [key, value] of upstream.headers) {
    if (key.toLowerCase() !== 'set-cookie') responseHeaders.set(key, value)
  }
  for (const cookie of upstream.headers.getSetCookie()) {
    responseHeaders.append('set-cookie', cookie)
  }
  return new Response(upstream.body, { status: upstream.status, headers: responseHeaders })
}
```

`web/src/middleware.ts`:

```ts
import { defineMiddleware } from 'astro:middleware'
import { me } from './lib/api'

export const onRequest = defineMiddleware(async (context, next) => {
  const path = context.url.pathname
  const guarded = (path.startsWith('/admin') && path !== '/admin/login') || path.startsWith('/preview')
  if (!guarded) return next()
  const cookie = context.request.headers.get('cookie') ?? ''
  try {
    await me(cookie)
    return next()
  } catch {
    return context.redirect('/admin/login')
  }
})
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run src/lib/api.test.ts`
Expected: 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): 添加类型化 API 客户端、代理端点与鉴权中间件"
```

---

### Task 5: Homepage (intro + year-grouped list), about, 404

**Files:**
- Modify: `web/src/pages/index.astro`
- Create: `web/src/pages/about.astro`
- Create: `web/src/pages/404.astro`

**Interfaces:**
- Consumes: `listPublished()`, `groupByYear`, `formatDay`, `recordPageview`, `Layout`.

- [ ] **Step 1: Implement the pages**

`web/src/pages/index.astro`:

```astro
---
import Layout from '../layouts/Layout.astro'
import { listPublished, recordPageview } from '../lib/api'
import { formatDay, groupByYear } from '../lib/date'

const articles = (await listPublished()).filter(
  (a): a is typeof a & { published_at: string } => a.published_at !== null,
)
const groups = groupByYear(articles)

await recordPageview({
  path: '/',
  article_id: null,
  ip: Astro.clientAddress,
  user_agent: Astro.request.headers.get('user-agent') ?? '',
  referer: Astro.request.headers.get('referer') ?? '',
})
---

<Layout title="首页">
  <section class="hero">
    <h1>你好，我是这里的主人 👋</h1>
    <p>
      欢迎来到我的小站。这里记录我的学习与生活。
      <a href="/about">了解更多 →</a>
    </p>
  </section>

  {groups.length === 0 && <p class="muted">还没有文章，敬请期待。</p>}

  {
    groups.map((group) => (
      <section class="year-group">
        <h2 class="year-title">{group.year}</h2>
        {group.items.map((item) => (
          <div class="post-item">
            <span class="post-date">{formatDay(item.published_at)}</span>
            <a href={`/posts/${item.slug}`}>{item.title}</a>
          </div>
        ))}
      </section>
    ))
  }
</Layout>
```

`web/src/pages/about.astro`:

```astro
---
import Layout from '../layouts/Layout.astro'
import { recordPageview } from '../lib/api'

await recordPageview({
  path: '/about',
  article_id: null,
  ip: Astro.clientAddress,
  user_agent: Astro.request.headers.get('user-agent') ?? '',
  referer: Astro.request.headers.get('referer') ?? '',
})
---

<Layout title="关于">
  <article class="card article-body">
    <h1>关于我</h1>
    <p>这里写一段自我介绍：你是谁，在做什么，为什么写这个博客。</p>
    <p>也可以放上你的联系方式、GitHub、兴趣爱好。随时回到 <a href="/">首页</a> 看看文章。</p>
  </article>
</Layout>
```

`web/src/pages/404.astro`:

```astro
---
import Layout from '../layouts/Layout.astro'
---

<Layout title="页面不存在">
  <div class="hero">
    <h1>404 · 页面走丢了</h1>
    <p>回 <a href="/">首页</a> 看看吧。</p>
  </div>
</Layout>
```

- [ ] **Step 2: Manual verification**

With the backend running (backend plan Task 10 smoke data present):

```bash
cd web && pnpm dev &
sleep 3
curl -s localhost:4321/ | grep -c 'post-item'      # ≥ 1 when a published article exists
curl -s localhost:4321/about | grep -o '关于我'
curl -s -o /dev/null -w '%{http_code}' localhost:4321/404  # 200 (Astro serves the page directly)
kill %1
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): 添加首页文章列表、关于页与 404 页"
```

---

### Task 6: Article detail page with math rendering and pageview recording

**Files:**
- Create: `web/src/pages/posts/[slug].astro`

**Interfaces:**
- Consumes: `getPublishedBySlug`, `renderMarkdown`, `recordPageview`, `Layout`.

- [ ] **Step 1: Implement the page**

`web/src/pages/posts/[slug].astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import { getPublishedBySlug, recordPageview } from '../../lib/api'
import { renderMarkdown } from '../../lib/markdown'

const { slug } = Astro.params
const article = slug === undefined ? null : await getPublishedBySlug(slug)
if (article === null) return Astro.redirect('/404')

const html = await renderMarkdown(article.markdown)

await recordPageview({
  path: Astro.url.pathname,
  article_id: article.id,
  ip: Astro.clientAddress,
  user_agent: Astro.request.headers.get('user-agent') ?? '',
  referer: Astro.request.headers.get('referer') ?? '',
})
---

<Layout title={article.title}>
  <article class="card">
    <h1 style="margin-top: 0">{article.title}</h1>
    <p class="muted" style="font-size: 14px">发布于 {article.published_at?.slice(0, 10)}</p>
    <div class="article-body" set:html={html} />
  </article>
</Layout>
```

- [ ] **Step 2: Manual verification**

Backend running with a published article whose markdown contains `$E=mc^2$`:

```bash
cd web && pnpm dev &
sleep 3
curl -s localhost:4321/posts/smoke-post | grep -o 'class="katex"' | head -1   # math rendered
curl -s -o /dev/null -w '%{http_code}\n' localhost:4321/posts/no-such        # 3xx redirect to /404
curl -s -b /tmp/zblog.jar localhost:8080/api/admin/stats/overview            # total_pv increased
kill %1
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): 添加文章详情页与访问上报"
```

---

### Task 7: Draft Preview route

**Files:**
- Create: `web/src/pages/preview/[slug].astro`

**Interfaces:**
- Consumes: `adminGetArticleBySlug` (cookie-forwarded), `renderMarkdown`. Auth enforced by `middleware.ts` (Task 4). No pageview recorded here.

- [ ] **Step 1: Implement the page**

`web/src/pages/preview/[slug].astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import { adminGetArticleBySlug } from '../../lib/api'
import { renderMarkdown } from '../../lib/markdown'

const cookie = Astro.request.headers.get('cookie') ?? ''
const { slug } = Astro.params
const article = slug === undefined ? null : await adminGetArticleBySlug(slug, cookie)
if (article === null) return Astro.redirect('/404')

const html = await renderMarkdown(article.markdown)
---

<Layout title={`预览 · ${article.title}`}>
  <div class="banner">草稿预览 — 此页面仅作者可见，发布前访客无法访问。</div>
  <article class="card">
    <h1 style="margin-top: 0">{article.title}</h1>
    <p class="muted" style="font-size: 14px">状态：{article.status === 'draft' ? '草稿' : '已发布'}</p>
    <div class="article-body" set:html={html} />
  </article>
</Layout>
```

- [ ] **Step 2: Manual verification**

Backend running; create a draft via admin API (do not publish):

```bash
cd web && pnpm dev &
sleep 3
curl -s -o /dev/null -w '%{http_code}\n' localhost:4321/preview/<draft-slug>              # 302 → /admin/login
curl -s -b /tmp/zblog.jar -L localhost:4321/preview/<draft-slug> | grep -o '草稿预览'    # renders with cookie
kill %1
```

(`/tmp/zblog.jar` from the backend smoke task, or re-login via `curl -c` against `localhost:4321/api/admin/login` to also verify the proxy forwards Set-Cookie.)

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): 添加草稿预览路由"
```

---

### Task 8: Admin login page and article list page

**Files:**
- Create: `web/src/components/LoginForm.vue`
- Create: `web/src/pages/admin/login.astro`
- Create: `web/src/pages/admin/index.astro`

**Interfaces:**
- Consumes: `login()`, `adminListArticles()`, `Layout`.
- Produces: `LoginForm.vue` (no props; posts via proxy; redirects to `/admin` on success, shows error on 401).

- [ ] **Step 1: Implement**

`web/src/components/LoginForm.vue`:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { login } from '../lib/api'

const password = ref('')
const error = ref('')
const submitting = ref(false)

async function submit() {
  error.value = ''
  submitting.value = true
  try {
    await login(password.value, { base: '' })
    window.location.assign('/admin')
  } catch {
    error.value = '密码错误，请重试。'
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <form class="card" style="max-width: 380px; margin: 60px auto" @submit.prevent="submit">
    <h1 style="font-size: 20px; margin-top: 0">后台登录</h1>
    <input v-model="password" class="input" type="password" placeholder="密码" autofocus />
    <p v-if="error" class="error-text">{{ error }}</p>
    <button class="btn" style="margin-top: 16px; width: 100%" :disabled="submitting" type="submit">
      {{ submitting ? '登录中…' : '登录' }}
    </button>
  </form>
</template>
```

`web/src/pages/admin/login.astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import LoginForm from '../../components/LoginForm.vue'
---

<Layout title="后台登录">
  <LoginForm client:load />
</Layout>
```

`web/src/pages/admin/index.astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import { adminListArticles } from '../../lib/api'

const cookie = Astro.request.headers.get('cookie') ?? ''
const articles = await adminListArticles(cookie)
---

<Layout title="文章管理">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
    <h1 style="font-size: 22px; margin: 0">文章管理</h1>
    <nav>
      <a class="btn btn-secondary" href="/admin/stats" style="margin-right: 10px">访问统计</a>
      <a class="btn" href="/admin/new">新建文章</a>
    </nav>
  </div>

  <div class="card">
    {articles.length === 0 && <p class="muted">还没有文章，点击「新建文章」开始写作。</p>}
    {
      articles.map((article) => (
        <div class="post-item">
          <span class="post-date">{article.updated_at.slice(0, 10)}</span>
          <a href={`/admin/edit/${article.id}`} style="flex: 1">{article.title}</a>
          {article.status === 'draft' ? (
            <>
              <span class="muted" style="font-size: 13px">草稿</span>
              <a href={`/preview/${article.slug}`} style="font-size: 13px">预览</a>
            </>
          ) : (
            <a href={`/posts/${article.slug}`} style="font-size: 13px">已发布</a>
          )}
        </div>
      ))
    }
  </div>
</Layout>
```

- [ ] **Step 2: Manual verification**

```bash
cd web && pnpm dev &
sleep 3
curl -s -o /dev/null -w '%{http_code}\n' localhost:4321/admin            # 302 to login
curl -s localhost:4321/admin/login | grep -o '后台登录'
curl -s -c /tmp/web.jar -X POST localhost:4321/api/admin/login -H 'content-type: application/json' -d '{"password":"dev-password"}'
curl -s -b /tmp/web.jar localhost:4321/admin | grep -o '文章管理'
kill %1
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): 添加后台登录页与文章管理列表页"
```

---

### Task 9: Markdown editor with Live Preview (new/edit pages)

**Files:**
- Create: `web/src/components/Editor.vue`
- Test: `web/src/components/Editor.test.ts`
- Create: `web/src/pages/admin/new.astro`
- Create: `web/src/pages/admin/edit/[id].astro`

**Interfaces:**
- Consumes: `adminCreateArticle`, `adminUpdateArticle`, `publishArticle`, `unpublishArticle`, `deleteArticle`, `renderMarkdown`, `type Article`.
- Produces: `Editor.vue` props `{ article?: Article }`. Behavior:
  - Local state `title/slug/markdown/html/status/currentId` (`currentId` starts from `article?.id ?? null`).
  - `watch(markdown)` re-renders preview immediately (KaTeX is fast enough; no debounce).
  - 保存草稿： create (when `currentId === null`, then `history.replaceState` to `/admin/edit/{id}`) or update.
  - 发布： save first, then publish; 转为草稿： save, then unpublish. After publish/unpublish, local `status` updates.
  - 删除： `confirm()` then delete, redirect `/admin`.
  - Slug input shown only when editing (create lets the server generate it).

- [ ] **Step 1: Write the failing test**

`web/src/components/Editor.test.ts`:

```ts
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
  it('renders markdown preview live as the author types', async () => {
    const wrapper = mount(Editor, { props: {} })
    await wrapper.find('textarea').setValue('# 标题 $x^2$')
    await vi.waitFor(() => {
      const preview = wrapper.find('[data-test="preview"]')
      expect(preview.html()).toContain('<h1>')
      expect(preview.html()).toContain('class="katex"')
    })
  })

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/components/Editor.test.ts`
Expected: FAIL — component missing.

- [ ] **Step 3: Write minimal implementation**

`web/src/components/Editor.vue`:

```vue
<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  adminCreateArticle,
  adminUpdateArticle,
  deleteArticle,
  publishArticle,
  unpublishArticle,
  type Article,
} from '../lib/api'
import { renderMarkdown } from '../lib/markdown'

type Props = { article?: Article }
const props = defineProps<Props>()

const title = ref(props.article?.title ?? '')
const slug = ref(props.article?.slug ?? '')
const markdown = ref(props.article?.markdown ?? '')
const html = ref('')
const status = ref<Article['status']>(props.article?.status ?? 'draft')
const currentId = ref<number | null>(props.article?.id ?? null)
const error = ref('')
const busy = ref(false)

watch(
  markdown,
  async (value) => {
    html.value = await renderMarkdown(value)
  },
  { immediate: true },
)

async function save(): Promise<boolean> {
  error.value = ''
  busy.value = true
  try {
    if (currentId.value === null) {
      const created = await adminCreateArticle(
        { title: title.value, markdown: markdown.value },
        { base: '' },
      )
      currentId.value = created.id
      slug.value = created.slug
      history.replaceState(null, '', `/admin/edit/${created.id}`)
    } else {
      await adminUpdateArticle(
        currentId.value,
        { title: title.value, slug: slug.value, markdown: markdown.value },
        { base: '' },
      )
    }
    return true
  } catch (e) {
    error.value = e instanceof Error ? e.message : '保存失败'
    return false
  } finally {
    busy.value = false
  }
}

async function publish() {
  if (!(await save()) || currentId.value === null) return
  const updated = await publishArticle(currentId.value, { base: '' })
  status.value = updated.status
}

async function unpublish() {
  if (!(await save()) || currentId.value === null) return
  const updated = await unpublishArticle(currentId.value, { base: '' })
  status.value = updated.status
}

async function remove() {
  if (currentId.value === null || !window.confirm('确认删除这篇文章？')) return
  await deleteArticle(currentId.value, { base: '' })
  window.location.assign('/admin')
}
</script>

<template>
  <div>
    <div style="display: flex; gap: 12px; margin-bottom: 12px">
      <input v-model="title" class="input" data-test="title" placeholder="标题" style="flex: 1" />
      <input
        v-if="currentId !== null"
        v-model="slug"
        class="input"
        placeholder="slug"
        style="width: 220px"
      />
    </div>

    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px">
      <textarea
        v-model="markdown"
        class="input"
        rows="22"
        placeholder="用 Markdown 写作，支持 $E=mc^2$ 这样的数学公式…"
        style="font-family: ui-monospace, Menlo, monospace; resize: vertical"
      />
      <div class="card article-body" data-test="preview" style="overflow-x: auto" v-html="html" />
    </div>

    <p v-if="error" class="error-text" data-test="error">{{ error }}</p>

    <div style="display: flex; gap: 10px; margin-top: 16px; align-items: center">
      <button class="btn btn-secondary" data-test="save" :disabled="busy" @click="save">
        保存草稿
      </button>
      <button v-if="status === 'draft'" class="btn" data-test="publish" :disabled="busy" @click="publish">
        发布
      </button>
      <button v-else class="btn btn-secondary" :disabled="busy" @click="unpublish">转为草稿</button>
      <button v-if="currentId !== null" class="btn btn-danger" :disabled="busy" @click="remove">
        删除
      </button>
      <span class="muted" style="font-size: 14px">
        当前状态：{{ status === 'draft' ? '草稿' : '已发布' }}
      </span>
      <a v-if="currentId !== null && status === 'draft'" :href="`/preview/${slug}`" target="_blank">
        预览 →
      </a>
    </div>
  </div>
</template>
```

`web/src/pages/admin/new.astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import Editor from '../../components/Editor.vue'
---

<Layout title="新建文章">
  <h1 style="font-size: 22px">新建文章</h1>
  <Editor client:load />
</Layout>
```

`web/src/pages/admin/edit/[id].astro`:

```astro
---
import Layout from '../../../layouts/Layout.astro'
import Editor from '../../../components/Editor.vue'
import { adminGetArticle } from '../../../lib/api'

const cookie = Astro.request.headers.get('cookie') ?? ''
const id = Number(Astro.params.id)
const article = Number.isInteger(id) ? await adminGetArticle(id, cookie).catch(() => null) : null
if (article === null) return Astro.redirect('/admin')
---

<Layout title={`编辑 · ${article.title}`}>
  <h1 style="font-size: 22px">编辑文章</h1>
  <Editor client:load article={article} />
</Layout>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run src/components/Editor.test.ts`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): 添加 Markdown 编辑器与新建/编辑页"
```

---

### Task 10: Stats dashboard and full-stack smoke

**Files:**
- Create: `web/src/components/StatsDashboard.vue`
- Test: `web/src/components/StatsDashboard.test.ts`
- Create: `web/src/pages/admin/stats.astro`

**Interfaces:**
- Consumes: `statsOverview`, `statsDaily`, `statsArticles`, `statsIps` — browser-side via `{ base: '' }` (cookie travels same-origin). `StatsDashboard.vue` has no props; loads all four on mount.

- [ ] **Step 1: Write the failing test**

`web/src/components/StatsDashboard.test.ts`:

```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && pnpm vitest run src/components/StatsDashboard.test.ts`
Expected: FAIL — component missing.

- [ ] **Step 3: Write minimal implementation**

`web/src/components/StatsDashboard.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  statsArticles,
  statsDaily,
  statsIps,
  statsOverview,
  type ArticleStat,
  type DailyStat,
  type IpStat,
  type StatsOverview,
} from '../lib/api'

const overview = ref<StatsOverview | null>(null)
const daily = ref<DailyStat[]>([])
const articles = ref<ArticleStat[]>([])
const ips = ref<IpStat[]>([])
const failed = ref(false)

onMounted(async () => {
  try {
    ;[overview.value, daily.value, articles.value, ips.value] = await Promise.all([
      statsOverview(''),
      statsDaily(30, ''),
      statsArticles(''),
      statsIps(20, ''),
    ])
  } catch {
    failed.value = true
  }
})
</script>

<template>
  <p v-if="failed" class="error-text">加载失败，请稍后重试。</p>
  <template v-else-if="overview">
    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 24px">
      <div class="card"><p class="muted" style="margin: 0">总浏览量</p><h3 style="margin: 4px 0 0">{{ overview.total_pv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">总访客数</p><h3 style="margin: 4px 0 0">{{ overview.total_uv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">今日浏览量</p><h3 style="margin: 4px 0 0">{{ overview.today_pv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">今日访客数</p><h3 style="margin: 4px 0 0">{{ overview.today_uv }}</h3></div>
    </div>

    <div class="card" style="margin-bottom: 24px">
      <h2 style="font-size: 18px; margin-top: 0">最近 30 天</h2>
      <div v-for="day in daily" :key="day.date" style="display: flex; gap: 12px; align-items: center; padding: 4px 0">
        <span class="post-date">{{ day.date }}</span>
        <div
          :style="{
            width: `${Math.min(100, (day.pv / Math.max(1, overview?.total_pv ?? 1)) * 400)}px`,
            height: '10px',
            background: 'var(--accent)',
            borderRadius: '5px',
          }"
        />
        <span class="muted" style="font-size: 13px">{{ day.pv }} 次 / {{ day.uv }} 人</span>
      </div>
      <p v-if="daily.length === 0" class="muted">暂无数据。</p>
    </div>

    <div class="card" style="margin-bottom: 24px">
      <h2 style="font-size: 18px; margin-top: 0">文章阅读量</h2>
      <div v-for="a in articles" :key="a.article_id" class="post-item">
        <span class="post-date">{{ a.pv }} 次</span>
        <a :href="`/posts/${a.slug}`" target="_blank">{{ a.title }}</a>
      </div>
      <p v-if="articles.length === 0" class="muted">暂无数据。</p>
    </div>

    <div class="card">
      <h2 style="font-size: 18px; margin-top: 0">Top IP</h2>
      <div v-for="row in ips" :key="row.ip" class="post-item">
        <span style="font-family: ui-monospace, Menlo, monospace">{{ row.ip }}</span>
        <span class="muted" style="margin-left: auto; font-size: 13px">
          {{ row.count }} 次 · 最近 {{ row.last_seen }}
        </span>
      </div>
      <p v-if="ips.length === 0" class="muted">暂无数据。</p>
    </div>
  </template>
</template>
```

Note: the four calls pass `''` as the `cookie` argument — the browser sends the real cookie same-origin automatically; the argument is required by the function signature. (SSR callers pass a real cookie.)

`web/src/pages/admin/stats.astro`:

```astro
---
import Layout from '../../layouts/Layout.astro'
import StatsDashboard from '../../components/StatsDashboard.vue'
---

<Layout title="访问统计">
  <h1 style="font-size: 22px">访问统计</h1>
  <StatsDashboard client:load />
</Layout>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && pnpm vitest run src/components/StatsDashboard.test.ts && pnpm vitest run`
Expected: component tests PASS; full suite (19 tests) PASS.

- [ ] **Step 5: Full-stack smoke (both processes, real browser flow)**

```bash
# terminal 1 — backend
cd server
HASH=$(cargo run --example hash_password -- 'dev-password' | tail -n1)
ZBLOG_PASSWORD_HASH="$HASH" ZBLOG_SESSION_SECRET="$(openssl rand -hex 32)$(openssl rand -hex 32)" cargo run &

# terminal 2 — frontend production build + run
cd web
pnpm build
RUST_API_URL=http://127.0.0.1:8080 node ./dist/server/entry.mjs &
sleep 3

# verify (note: built SSR, not dev server)
curl -s localhost:4321/ | grep -o '你好，我是这里的主人'
curl -s -c /tmp/smoke.jar -X POST localhost:4321/api/admin/login -H 'content-type: application/json' -d '{"password":"dev-password"}'
curl -s -b /tmp/smoke.jar -X POST localhost:4321/api/admin/articles -H 'content-type: application/json' \
  -d '{"title":"数学之美","markdown":"# 公式\n\n$e^{i\\pi}+1=0$"}'
curl -s -b /tmp/smoke.jar -X POST localhost:4321/api/admin/articles/1/publish
curl -s localhost:4321/ | grep -o '数学之美'
curl -s localhost:4321/posts/shu-xue-zhi-mei >/dev/null  # slug for Chinese title is "post"
curl -s localhost:4321/posts/post | grep -o 'class="katex"' | head -1
curl -s -b /tmp/smoke.jar localhost:8080/api/admin/stats/overview   # total_pv ≥ 2
curl -s -b /tmp/smoke.jar localhost:4321/admin | grep -o '文章管理'
curl -s -b /tmp/smoke.jar -o /dev/null -w '%{http_code}\n' localhost:4321/admin/stats   # 200

kill %1 %2
rm -f server/zblog.db /tmp/smoke.jar
```

Expected: every grep finds its string; stats 200; math rendered on the built site.

- [ ] **Step 6: Final test sweep and commit**

Run: `cd web && pnpm vitest run && pnpm astro check` (install `@astrojs/check` first if prompted: `pnpm add -D @astrojs/check typescript`)
Expected: all green.

```bash
git add web/
git commit -m "feat(web): 添加访问统计看板并完成全栈联调"
```
