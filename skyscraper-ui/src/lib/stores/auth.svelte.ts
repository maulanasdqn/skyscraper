const DEFAULT_ADMIN_KEY = 'sk_admin_test'
const DEFAULT_COSMIUM_URL = 'http://localhost:3000'
const DEFAULT_PARALLAX_URL = 'http://localhost:3001'

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
  webhookUrl: 'skyscraper_webhook_url',
} as const

let adminKey = $state(loadKey(KEYS.admin, DEFAULT_ADMIN_KEY))
let tenantApiKey = $state(loadKey(KEYS.tenant, ''))
let cosmiumUrl = $state(loadKey(KEYS.cosmium, DEFAULT_COSMIUM_URL))
let parallaxUrl = $state(loadKey(KEYS.parallax, DEFAULT_PARALLAX_URL))
let cosmiumApiKey = $state(loadKey(KEYS.cosmiumApiKey, 'dev-key'))
let webhookUrl = $state(loadKey(KEYS.webhookUrl, ''))
let provisioning = $state(false)

export function getAdminKey() { return adminKey }
export function setAdminKey(value: string) {
  adminKey = value
  persist(KEYS.admin, value)
}

export function getTenantApiKey() { return tenantApiKey }
export function setTenantApiKey(value: string) {
  tenantApiKey = value
  persist(KEYS.tenant, value)
}

export function getCosmiumUrl() { return cosmiumUrl }
export function setCosmiumUrl(value: string) {
  cosmiumUrl = value.replace(/\/+$/, '')
  persist(KEYS.cosmium, cosmiumUrl)
}

export function getParallaxUrl() { return parallaxUrl }
export function setParallaxUrl(value: string) {
  parallaxUrl = value.replace(/\/+$/, '')
  persist(KEYS.parallax, parallaxUrl)
}

export function getCosmiumApiKey() { return cosmiumApiKey }
export function setCosmiumApiKey(value: string) {
  cosmiumApiKey = value
  persist(KEYS.cosmiumApiKey, value)
}

export function getWebhookUrl() { return webhookUrl }
export function setWebhookUrl(value: string) {
  webhookUrl = value
  persist(KEYS.webhookUrl, value)
}

export function isAdminConfigured(): boolean { return adminKey.length > 0 }
export function isTenantConfigured(): boolean { return tenantApiKey.length > 0 }
export function isProvisioning(): boolean { return provisioning }

export async function ensureTenant(): Promise<string> {
  if (tenantApiKey) return tenantApiKey

  if (provisioning) {
    while (provisioning) await new Promise((r) => setTimeout(r, 100))
    return tenantApiKey
  }

  provisioning = true
  try {
    const key = adminKey || DEFAULT_ADMIN_KEY
    const res = await fetch('/api/v1/admin/tenants', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Api-Key': key },
      body: JSON.stringify({
        email: `auto@skyscraper.local`,
        plan: 'business',
        rate_limit_rpm: 600,
        concurrent_limit: 50,
      }),
    })
    if (!res.ok) {
      const list = await fetch('/api/v1/admin/tenants', {
        headers: { 'X-Api-Key': key },
      })
      if (list.ok) {
        const tenants = await list.json()
        if (tenants.length > 0) {
          setTenantApiKey(tenants[0].api_key)
          return tenantApiKey
        }
      }
      throw new Error('Failed to provision tenant')
    }
    const tenant = await res.json()
    setTenantApiKey(tenant.api_key)
    return tenantApiKey
  } finally {
    provisioning = false
  }
}
