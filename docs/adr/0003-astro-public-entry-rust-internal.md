# Astro (Node) is the only public entry point; Rust API listens on localhost

Deployment runs two processes on one host: the Astro SSR server (Node) faces the public internet and calls the Rust (axum) API over localhost; the Rust process binds to an internal address only. We rejected an nginx front (extra component to operate, no benefit at this scale) and Rust-as-reverse-proxy (extra code, no benefit). In development, the Astro dev server proxies `/api` to the Rust process.

**Consequences**: All browser traffic — public pages, admin pages, and API calls — enters through the Node process. Admin authentication (password + signed session cookie) is enforced on API routes regardless, since "internal only" is a deployment posture, not a security boundary.
