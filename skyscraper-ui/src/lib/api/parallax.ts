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

function api(_baseUrl: string, path: string): string {
  return `/parallax${path}`
}

export interface ParallaxCarrier {
  id: string
  name: string
  country: string
  upstream_addr: string
  online: boolean
}

export const getParallaxCarriers = (baseUrl: string) =>
  request<ParallaxCarrier[]>(api(baseUrl, '/carriers'))

export interface ParallaxUser {
  id: string
  email: string
  api_key: string
  plan_id: string | null
  active: boolean
}

export const listUsers = (baseUrl: string) =>
  request<ParallaxUser[]>(api(baseUrl, '/users'))

export const createUser = (baseUrl: string, data: Record<string, unknown>) =>
  request<ParallaxUser>(api(baseUrl, '/users'), {
    method: 'POST',
    headers: JSON_HEADERS,
    body: JSON.stringify(data),
  })

export const deleteUser = (baseUrl: string, id: string) =>
  request<{ deleted: boolean }>(api(baseUrl, `/users/${id}`), {
    method: 'DELETE',
  })

export const toggleUser = (baseUrl: string, id: string) =>
  request<ParallaxUser>(api(baseUrl, `/users/${id}/toggle`), {
    method: 'PATCH',
  })

export const regenerateUserKey = (baseUrl: string, id: string) =>
  request<{ api_key: string }>(api(baseUrl, `/users/${id}/regenerate`), {
    method: 'POST',
  })

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
  request<ParallaxSession[]>(api(baseUrl, '/sessions'))
