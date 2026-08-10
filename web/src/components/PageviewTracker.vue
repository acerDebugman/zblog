<script setup lang="ts">
import { onMounted } from 'vue'
import { recordPageview } from '../lib/api'

onMounted(() => {
  void recordPageview({ path: location.pathname, article_id: null })
})
</script>

<template>
  <span style="display: none" aria-hidden="true" />
</template>
