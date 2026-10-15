import { get, post, patch, del } from './client'
import type {
  Tenant,
  Job,
  ScrapeResult,
  QueueStats,
  Metrics,
  CacheStats,
  Carrier,
  PaginatedResponse,
  UsageInfo,
} from '../types'

// ── Health ──────────────────────────────────────────────

export const getHealth = () => get<{ status: string; version: string }>('/health')

export const getReadiness = () => get<{ status: string }>('/health/ready')

// ── Tenants (admin) ─────────────────────────────────────

export const listTenants = (adminKey: string) =>
  get<Tenant[]>('/admin/tenants', adminKey)

export const createTenant = (adminKey: string, data: Partial<Tenant>) =>
  post<Tenant>('/admin/tenants', data, adminKey)

export const updateTenant = (adminKey: string, id: string, data: Partial<Tenant>) =>
  patch<Tenant>(`/admin/tenants/${id}`, data, adminKey)

export const regenerateKey = (adminKey: string, id: string) =>
  post<Tenant>(`/admin/tenants/${id}/regenerate-key`, {}, adminKey)

// ── Jobs ────────────────────────────────────────────────

export interface ListJobsParams {
  page?: number
  per_page?: number
  status?: string
}

export const listJobs = (apiKey: string, params: ListJobsParams = {}) => {
  const q = new URLSearchParams()
  if (params.page) q.set('page', String(params.page))
  if (params.per_page) q.set('per_page', String(params.per_page))
  if (params.status) q.set('status', params.status)
  const qs = q.toString()
  return get<PaginatedResponse<Job>>(`/jobs${qs ? `?${qs}` : ''}`, apiKey)
}

export const getJob = (apiKey: string, id: string) =>
  get<Job>(`/jobs/${id}`, apiKey)

export const cancelJob = (apiKey: string, id: string) =>
  del<{ cancelled: boolean }>(`/jobs/${id}`, apiKey)

export const retryJob = (apiKey: string, id: string) =>
  post<Job>(`/jobs/${id}/retry`, {}, apiKey)

export const updatePriority = (apiKey: string, id: string, priority: number) =>
  patch<{ updated: boolean }>(`/jobs/${id}/priority`, { priority }, apiKey)

// ── Scrape ──────────────────────────────────────────────

export interface SubmitJobData {
  url: string
  extract?: string[]
  script?: string
  screenshot?: boolean
  include_html?: boolean
  wait_ms?: number
  headful?: boolean
  geo?: string
  priority?: number
  max_retries?: number
  scheduled_at?: string
  proxy_strategy?: string
  profile_strategy?: string
}

export const submitJob = (apiKey: string, data: SubmitJobData) =>
  post<Job>('/jobs', data, apiKey)

export const submitBatch = (apiKey: string, jobs: SubmitJobData[]) =>
  post<{ job_ids: string[] }>('/jobs/batch', { jobs }, apiKey)

export const scrapeSync = (apiKey: string, data: SubmitJobData, timeoutMs = 60000) =>
  post<{ job_id: string; status: string; result?: ScrapeResult }>(
    `/scrape?timeout_ms=${timeoutMs}`,
    data,
    apiKey,
  )

// ── Results ─────────────────────────────────────────────

export const getResult = (apiKey: string, id: string) =>
  get<ScrapeResult>(`/results/${id}`, apiKey)

export const getScreenshotUrl = (id: string) => `/api/v1/results/${id}/screenshot`

// ── Usage ───────────────────────────────────────────────

export const getUsage = (apiKey: string) =>
  get<UsageInfo>('/usage', apiKey)

// ── Admin ───────────────────────────────────────────────

export const getQueueStats = (adminKey: string) =>
  get<QueueStats>('/admin/queue/stats', adminKey)

export const retryDead = (adminKey: string) =>
  post<{ retried: number }>('/admin/queue/retry-dead', {}, adminKey)

export const listCarriers = (adminKey: string) =>
  get<Carrier[]>('/admin/carriers', adminKey)

export const getMetrics = (adminKey: string) =>
  get<Metrics>('/admin/metrics', adminKey)

export const getCacheStats = (adminKey: string) =>
  get<CacheStats>('/admin/cache', adminKey)

export const flushCache = (adminKey: string) =>
  del<{ flushed: boolean }>('/admin/cache', adminKey)

export interface ListAllJobsParams extends ListJobsParams {
  tenant_id?: string
}

export const listAllJobs = (adminKey: string, params: ListAllJobsParams = {}) => {
  const q = new URLSearchParams()
  if (params.page) q.set('page', String(params.page))
  if (params.per_page) q.set('per_page', String(params.per_page))
  if (params.status) q.set('status', params.status)
  if (params.tenant_id) q.set('tenant_id', params.tenant_id)
  const qs = q.toString()
  return get<PaginatedResponse<Job>>(`/admin/jobs${qs ? `?${qs}` : ''}`, adminKey)
}
