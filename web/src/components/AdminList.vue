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
