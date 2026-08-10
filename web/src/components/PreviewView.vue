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
