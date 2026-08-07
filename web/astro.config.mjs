import { defineConfig } from 'astro/config'
import vue from '@astrojs/vue'
import node from '@astrojs/node'

export default defineConfig({
  output: 'server',
  adapter: node({ mode: 'standalone' }),
  integrations: [vue()],
})
