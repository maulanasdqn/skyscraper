import { ApiError } from './client'

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init)
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new ApiError(res.status, body.error || res.statusText)
  }
  return res.json()
}

const JSON_HEADERS: HeadersInit = { 'Content-Type': 'application/json' }

// ── Carriers ────────────────────────────────────────────

export interface ParallaxCarrier {
  id: string
  name: string
  country: string
  upstream_addr: string
  online: boolean
}

export const getParallaxCarriers = (baseUrl: string) =>
  request<ParallaxCarrier[]>(`${baseUrl}/api/carriers`)

// ── Users ───────────────────────────────────────────────

export interface ParallaxUser {
  id: string
  email: string
  api_key: string
  plan_id: string | null
  active: boolean
}

export const listUsers = (baseUrl: string) =>
  request<ParallaxUser[]>(`${baseUrl}/api/users`)

export const createUser = (baseUrl: string, data: Record<string, unknown>) =>
  request<ParallaxUser>(`${baseUrl}/api/users`, {
    method: 'POST',
    headers: JSON_HEADERS,
    body: JSON.stringify(data),
  })

export const deleteUser = (baseUrl: string, id: string) =>
  request<{ deleted: boolean }>(`${baseUrl}/api/users/${id}`, {
    method: 'DELETE',
  })

export const toggleUser = (baseUrl: string, id: string) =>
  request<ParallaxUser>(`${baseUrl}/api/users/${id}/toggle`, {
    method: 'PATCH',
  })

export const regenerateUserKey = (baseUrl: string, id: string) =>
  request<{ api_key: string }>(`${baseUrl}/api/users/${id}/regenerate`, {
    method: 'POST',
  })

// ── Sessions ────────────────────────────────────────────

export interface ParallaxSession {
  id: string
  user_id: string
  carrier_id: string
  bytes_up: number
  bytes_down: number
  started_at: string
  active: boolean
}

export const listSessions = (baseUrl: string) =>
  request<ParallaxSession[]>(`${baseUrl}/api/sessions`)
