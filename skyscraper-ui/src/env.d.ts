/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_SKYSCRAPER_URL: string
  readonly VITE_COSMIUM_URL: string
  readonly VITE_PARALLAX_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
