<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { listPublished, type Article } from '../lib/api'
import { formatDay, groupByYear } from '../lib/date'

type DatedArticle = Article & { published_at: string }

const groups = ref<{ year: string; items: DatedArticle[] }[]>([])
const loaded = ref(false)

onMounted(async () => {
  const articles = (await listPublished()).filter(
    (a): a is DatedArticle => a.published_at !== null,
  )
  groups.value = groupByYear(articles)
  loaded.value = true
})
</script>

<template>
  <p v-if="loaded && groups.length === 0" class="muted">还没有文章，敬请期待。</p>
  <section v-for="group in groups" :key="group.year" class="year-group">
    <h2 class="year-title">{{ group.year }}</h2>
    <div v-for="item in group.items" :key="item.slug" class="post-item">
      <span class="post-date">{{ formatDay(item.published_at) }}</span>
      <a :href="`/posts/${item.slug}`">{{ item.title }}</a>
    </div>
  </section>
</template>
