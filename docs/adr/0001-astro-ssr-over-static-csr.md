---
status: superseded by ADR-0004
---

# Astro SSR over static build with client-side fetching

The public frontend is served by Astro in SSR mode (Node adapter), fetching article data from the Rust API at request time. We rejected the simpler-deploy alternative (static Astro build + Vue islands fetching the API in the browser) because article pages would then be client-rendered: weaker SEO and first-paint flicker, which matters for a blog whose main job is being read. SSR also gives us a single server-side Markdown/math rendering pipeline (remark/rehype + KaTeX) that the admin Live Preview reuses. The cost — a long-running Node process in production — is accepted and reflected in ADR-0003.

**Consequences**: Production must run a Node process for Astro alongside the Rust API. Any change to the rendering pipeline must keep the admin preview and the SSR renderer on the same code path.
