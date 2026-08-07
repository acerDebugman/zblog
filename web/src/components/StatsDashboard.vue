<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  statsArticles,
  statsDaily,
  statsIps,
  statsOverview,
  type ArticleStat,
  type DailyStat,
  type IpStat,
  type StatsOverview,
} from '../lib/api'

const overview = ref<StatsOverview | null>(null)
const daily = ref<DailyStat[]>([])
const articles = ref<ArticleStat[]>([])
const ips = ref<IpStat[]>([])
const failed = ref(false)

onMounted(async () => {
  try {
    ;[overview.value, daily.value, articles.value, ips.value] = await Promise.all([
      statsOverview(''),
      statsDaily(30, ''),
      statsArticles(''),
      statsIps(20, ''),
    ])
  } catch {
    failed.value = true
  }
})
</script>

<template>
  <p v-if="failed" class="error-text">加载失败，请稍后重试。</p>
  <template v-else-if="overview">
    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 24px">
      <div class="card"><p class="muted" style="margin: 0">总浏览量</p><h3 style="margin: 4px 0 0">{{ overview.total_pv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">总访客数</p><h3 style="margin: 4px 0 0">{{ overview.total_uv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">今日浏览量</p><h3 style="margin: 4px 0 0">{{ overview.today_pv }}</h3></div>
      <div class="card"><p class="muted" style="margin: 0">今日访客数</p><h3 style="margin: 4px 0 0">{{ overview.today_uv }}</h3></div>
    </div>

    <div class="card" style="margin-bottom: 24px">
      <h2 style="font-size: 18px; margin-top: 0">最近 30 天</h2>
      <div v-for="day in daily" :key="day.date" style="display: flex; gap: 12px; align-items: center; padding: 4px 0">
        <span class="post-date">{{ day.date }}</span>
        <div
          :style="{
            width: `${Math.min(100, (day.pv / Math.max(1, overview?.total_pv ?? 1)) * 400)}px`,
            height: '10px',
            background: 'var(--accent)',
            borderRadius: '5px',
          }"
        />
        <span class="muted" style="font-size: 13px">{{ day.pv }} 次 / {{ day.uv }} 人</span>
      </div>
      <p v-if="daily.length === 0" class="muted">暂无数据。</p>
    </div>

    <div class="card" style="margin-bottom: 24px">
      <h2 style="font-size: 18px; margin-top: 0">文章阅读量</h2>
      <div v-for="a in articles" :key="a.article_id" class="post-item">
        <span class="post-date">{{ a.pv }} 次</span>
        <a :href="`/posts/${a.slug}`" target="_blank">{{ a.title }}</a>
      </div>
      <p v-if="articles.length === 0" class="muted">暂无数据。</p>
    </div>

    <div class="card">
      <h2 style="font-size: 18px; margin-top: 0">Top IP</h2>
      <div v-for="row in ips" :key="row.ip" class="post-item">
        <span style="font-family: ui-monospace, Menlo, monospace">{{ row.ip }}</span>
        <span class="muted" style="margin-left: auto; font-size: 13px">
          {{ row.count }} 次 · 最近 {{ row.last_seen }}
        </span>
      </div>
      <p v-if="ips.length === 0" class="muted">暂无数据。</p>
    </div>
  </template>
</template>
