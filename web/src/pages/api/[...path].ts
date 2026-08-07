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
