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
