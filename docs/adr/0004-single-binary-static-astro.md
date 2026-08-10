---
status: accepted
supersedes: ADR-0001, ADR-0003
---

# Single-binary deployment: static Astro build embedded in the Rust server

The whole app now deploys as one Rust binary. Astro became a build-time-only tool (`output: 'static'`); `rust-embed` embeds `web/dist` into the binary, and axum serves the embedded assets with SPA fallback alongside the API. All pages — including article pages — render client-side from the API. We accept the SEO/first-paint trade-off that ADR-0001 originally rejected: for this single-author personal blog, deployment simplicity (one file, no Node process, no proxy layer) outweighs search indexing.

This reverses two earlier decisions; they remain in the record for context:

- ADR-0001 (Astro SSR over static CSR) — superseded; the Markdown/KaTeX pipeline now runs in the browser, still shared between article rendering and the editor's Live Preview.
- ADR-0003 (Astro public entry, Rust internal) — superseded; Rust is the only process and the public entry. The Astro middleware and the `/api/*` proxy endpoint disappear; browsers call the Rust API same-origin.

**Consequences**:

- Pageview IP/UA/referer come from the direct connection (`ConnectInfo` + request headers) instead of Astro forwarding — one less hop and more accurate; `POST /api/pageviews` is now genuinely internet-reachable and its payload narrowed to `{path, article_id}` (accepted: stats are self-reference; hardening backlog carries rate-limit/bot-filter).
- Admin/preview pages are public static shells guarded client-side (`/api/admin/me` probe → redirect to login); the data boundary remains the API's `require_auth`.
- Build order matters: `pnpm build` must run before `cargo build`; fresh clones need a placeholder `web/dist` to compile the backend alone.
