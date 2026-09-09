<script setup lang="ts">
import { ref } from 'vue'
import { login, LoginLockedError } from '../lib/api'

const password = ref('')
const error = ref('')
const submitting = ref(false)

function lockoutMessage(retryAfter: number): string {
  const minutes = Math.floor(retryAfter / 60)
  const seconds = Math.ceil(retryAfter % 60)
  return minutes > 0
    ? `尝试次数过多，请 ${minutes} 分 ${seconds} 秒后重试。`
    : `尝试次数过多，请 ${seconds} 秒后重试。`
}

async function submit() {
  error.value = ''
  submitting.value = true
  try {
    await login(password.value)
    window.location.assign('/admin')
  } catch (e) {
    error.value = e instanceof LoginLockedError ? lockoutMessage(e.retryAfter) : '密码错误，请重试。'
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <form class="card" style="max-width: 380px; margin: 60px auto" @submit.prevent="submit">
    <h1 style="font-size: 20px; margin-top: 0">后台登录</h1>
    <input v-model="password" class="input" type="password" placeholder="密码" autofocus />
    <p v-if="error" class="error-text">{{ error }}</p>
    <button class="btn" style="margin-top: 16px; width: 100%" :disabled="submitting" type="submit">
      {{ submitting ? '登录中…' : '登录' }}
    </button>
  </form>
</template>
