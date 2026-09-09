<script setup lang="ts">
import { ref } from 'vue'
import {
  adminCreateArticle,
  adminUpdateArticle,
  deleteArticle,
  publishArticle,
  unpublishArticle,
  type Article,
} from '../lib/api'

type Props = { article?: Article }
const props = defineProps<Props>()

const title = ref(props.article?.title ?? '')
const slug = ref(props.article?.slug ?? '')
const markdown = ref(props.article?.markdown ?? '')
const status = ref<Article['status']>(props.article?.status ?? 'draft')
const currentId = ref<number | null>(props.article?.id ?? null)
const error = ref('')
const busy = ref(false)

async function save(): Promise<boolean> {
  error.value = ''
  busy.value = true
  try {
    if (currentId.value === null) {
      const created = await adminCreateArticle({
        title: title.value,
        markdown: markdown.value,
      })
      currentId.value = created.id
      slug.value = created.slug
      history.replaceState(null, '', `/admin/edit/${created.id}`)
    } else {
      await adminUpdateArticle(currentId.value, {
        title: title.value,
        slug: slug.value,
        markdown: markdown.value,
      })
    }
    return true
  } catch (e) {
    error.value = e instanceof Error ? e.message : '保存失败'
    return false
  } finally {
    busy.value = false
  }
}

async function publish() {
  if (!(await save()) || currentId.value === null) return
  const updated = await publishArticle(currentId.value)
  status.value = updated.status
}

async function unpublish() {
  if (!(await save()) || currentId.value === null) return
  const updated = await unpublishArticle(currentId.value)
  status.value = updated.status
}

async function remove() {
  if (currentId.value === null || !window.confirm('确认删除这篇文章？')) return
  await deleteArticle(currentId.value)
  window.location.assign('/admin')
}
</script>

<template>
  <div>
    <div style="display: flex; gap: 12px; margin-bottom: 12px">
      <input v-model="title" class="input" data-test="title" placeholder="标题" style="flex: 1" />
      <input
        v-if="currentId !== null"
        v-model="slug"
        class="input"
        placeholder="slug"
        style="width: 220px"
      />
    </div>

    <textarea
      v-model="markdown"
      class="input editor-input"
      placeholder="用 Markdown 写作，支持 $E=mc^2$ 这样的数学公式…"
    />

    <p v-if="error" class="error-text" data-test="error">{{ error }}</p>

    <div style="display: flex; gap: 10px; margin-top: 16px; align-items: center">
      <button class="btn btn-secondary" data-test="save" :disabled="busy" @click="save">
        保存草稿
      </button>
      <button v-if="status === 'draft'" class="btn" data-test="publish" :disabled="busy" @click="publish">
        发布
      </button>
      <button v-else class="btn btn-secondary" :disabled="busy" @click="unpublish">转为草稿</button>
      <button v-if="currentId !== null" class="btn btn-danger" :disabled="busy" @click="remove">
        删除
      </button>
      <span class="muted" style="font-size: 14px">
        当前状态：{{ status === 'draft' ? '草稿' : '已发布' }}
      </span>
      <a v-if="currentId !== null && status === 'draft'" :href="`/preview/${slug}`" target="_blank">
        预览 →
      </a>
    </div>
  </div>
</template>

<style scoped>
.editor-input {
  font-family: ui-monospace, Menlo, monospace;
  resize: none;
  height: calc(100vh - 320px);
  min-height: 400px;
}
</style>
