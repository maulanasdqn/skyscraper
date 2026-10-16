export interface Tenant {
  id: string
  email: string
  api_key?: string
  plan: TenantPlan
  active: boolean
  rate_limit_rpm: number
  concurrent_limit: number
  webhook_url: string | null
  created_at: string
}

export type TenantPlan = 'free' | 'starter' | 'pro' | 'business'

export interface Job {
  id: string
  tenant_id: string
  status: JobStatus
  priority: number
  request: ScrapeRequest
  result: ScrapeResult | null
  carrier_id: string | null
  profile_name: string | null
  attempts: number
  max_retries: number
  scheduled_at: string | null
  started_at: string | null
  completed_at: string | null
  error: string | null
  created_at: string
}

export type JobStatus = 'pending' | 'scheduled' | 'running' | 'completed' | 'failed' | 'dead'

export interface ScrapeRequest {
  url: string
  extract: string[]
  script?: string
  workflow: unknown[]
  screenshot: boolean
  include_html: boolean
  wait_ms: number
  headful: boolean
  geo?: string
  proxy_strategy: string
  profile_strategy: string
}

export interface ScrapeResult {
  final_url: string
  http_status: number
  html_length: number
  html: string | null
  extracted: Record<string, unknown>
  screenshot_base64: string | null
  blocked: boolean
  proxy_used: string | null
  profile_used: string
  elapsed_ms: number
  attempts: number
}

export interface Carrier {
  id: string
  name: string
  country: string
  upstream_addr: string
  online: boolean
}

export interface QueueStats {
  pending: number
  processing: number
  scheduled: number
  dead: number
}

export interface Metrics {
  jobs_processed: number
  jobs_failed: number
  jobs_timed_out: number
  cache_hits: number
  cache_misses: number
  scrape_total_ms: number
  webhook_deliveries: number
  webhook_failures: number
  dedup_coalesced: number
  domain_throttled: number
}

export interface CacheStats {
  cached_urls: number
  ttl_secs: number
}

export interface PaginatedResponse<T> {
  data: T[]
  pagination: {
    page: number
    per_page: number
    total: number
    total_pages: number
  }
}

export interface UsageInfo {
  total_bytes: number
  period_start: string
  period_end: string
}

export interface CrawlResponse {
  crawl_id: string
  job_ids: string[]
  pages_discovered: number
}
