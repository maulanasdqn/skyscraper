function loadKey(key: string, fallback: string): string {
  try {
    return localStorage.getItem(key) || fallback
  } catch {
    return fallback
  }
}

function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // storage unavailable
  }
}

const KEYS = {
  admin: 'skyscraper_admin_key',
  tenant: 'skyscraper_tenant_key',
  cosmium: 'skyscraper_cosmium_url',
  parallax: 'skyscraper_parallax_url',
  cosmiumApiKey: 'skyscraper_cosmium_api_key',
} as const

let adminKey = $state(loadKey(KEYS.admin, ''))
let tenantApiKey = $state(loadKey(KEYS.tenant, ''))
let cosmiumUrl = $state(loadKey(KEYS.cosmium, 'http://localhost:3000'))
let parallaxUrl = $state(loadKey(KEYS.parallax, 'http://localhost:3001'))
let cosmiumApiKey = $state(loadKey(KEYS.cosmiumApiKey, ''))

export function getAdminKey() {
  return adminKey
}

export function setAdminKey(value: string) {
  adminKey = value
  persist(KEYS.admin, value)
}

export function getTenantApiKey() {
  return tenantApiKey
}

export function setTenantApiKey(value: string) {
  tenantApiKey = value
  persist(KEYS.tenant, value)
}

export function getCosmiumUrl() {
  return cosmiumUrl
}

export function setCosmiumUrl(value: string) {
  cosmiumUrl = value.replace(/\/+$/, '')
  persist(KEYS.cosmium, cosmiumUrl)
}

export function getParallaxUrl() {
  return parallaxUrl
}

export function setParallaxUrl(value: string) {
  parallaxUrl = value.replace(/\/+$/, '')
  persist(KEYS.parallax, parallaxUrl)
}

export function getCosmiumApiKey() {
  return cosmiumApiKey
}

export function setCosmiumApiKey(value: string) {
  cosmiumApiKey = value
  persist(KEYS.cosmiumApiKey, value)
}

export function isAdminConfigured(): boolean {
  return adminKey.length > 0
}

export function isTenantConfigured(): boolean {
  return tenantApiKey.length > 0
}
