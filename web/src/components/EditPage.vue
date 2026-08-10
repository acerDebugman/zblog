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
