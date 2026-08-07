/// <reference path="../.astro/types.d.ts" />

interface ImportMetaEnv {
  readonly RUST_API_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
