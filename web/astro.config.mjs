import { defineConfig } from 'astro/config'
import vue from '@astrojs/vue'

export default defineConfig({
  integrations: [vue()],
  vite: {
    server: {
      proxy: { '/api': 'http://127.0.0.1:8080' },
    },
  },
})
