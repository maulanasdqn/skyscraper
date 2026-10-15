import { ApiError } from './client'

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init)
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new ApiError(res.status, body.error || res.statusText)
  }
  return res.json()
}

function headers(apiKey?: string): HeadersInit {
  const h: HeadersInit = { 'Content-Type': 'application/json' }
  if (apiKey) h['X-Api-Key'] = apiKey
  return h
}

export interface CosmiumHealth {
  status: string
  version: string
  binary_found: boolean
  llm_configured: boolean
}

export interface ProfileSummary {
  name: string
  user_agent?: string
  platform?: string
}

function base(url: string): string {
  if (url.startsWith('http')) return url
  return ''
}

export const getCosmiumHealth = (baseUrl: string, apiKey?: string) =>
  request<CosmiumHealth>(`${base(baseUrl)}/cosmium/health`, { headers: headers(apiKey) })

export const listProfiles = (baseUrl: string, apiKey?: string) =>
  request<{ profiles: string[] }>(`${base(baseUrl)}/cosmium/v1/profiles`, {
    headers: headers(apiKey),
  })

export const getProfileSummaries = (baseUrl: string, apiKey?: string) =>
  request<ProfileSummary[]>(`${base(baseUrl)}/cosmium/v1/profiles/summaries`, {
    headers: headers(apiKey),
  })

export const getProfile = (baseUrl: string, name: string, apiKey?: string) =>
  request<Record<string, unknown>>(`${base(baseUrl)}/cosmium/v1/profiles/${encodeURIComponent(name)}`, {
    headers: headers(apiKey),
  })

export const deleteProfile = (baseUrl: string, name: string, apiKey?: string) =>
  request<{ deleted: boolean }>(`${base(baseUrl)}/cosmium/v1/profiles/${encodeURIComponent(name)}`, {
    method: 'DELETE',
    headers: headers(apiKey),
  })

export const saveProfile = (
  baseUrl: string,
  data: { name: string; profile: unknown },
  apiKey?: string,
) =>
  request<{ saved: boolean }>(`${base(baseUrl)}/cosmium/v1/profiles/save`, {
    method: 'POST',
    headers: headers(apiKey),
    body: JSON.stringify(data),
  })

export const generateProfile = (
  baseUrl: string,
  persona: string,
  name: string,
  apiKey?: string,
) =>
  request<Record<string, unknown>>(`${base(baseUrl)}/cosmium/v1/profiles/generate`, {
    method: 'POST',
    headers: headers(apiKey),
    body: JSON.stringify({ persona, name }),
  })

export const runFingerprintTest = (baseUrl: string, apiKey?: string) =>
  request<Record<string, unknown>>(`${base(baseUrl)}/cosmium/v1/tests/fingerprint`, {
    method: 'POST',
    headers: headers(apiKey),
  })

export const runStealthTest = (baseUrl: string, apiKey?: string) =>
  request<Record<string, unknown>>(`${base(baseUrl)}/cosmium/v1/tests/stealth`, {
    method: 'POST',
    headers: headers(apiKey),
  })
