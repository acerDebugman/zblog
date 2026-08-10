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
