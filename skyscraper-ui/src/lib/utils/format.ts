import type { JobStatus, TenantPlan } from '@lib/types'

export function formatDate(iso: string | null): string {
  if (!iso) return '—'
  const date = new Date(iso)
  const now = Date.now()
  const diff = now - date.getTime()

  if (diff < 0) return 'in the future'
  if (diff < 60_000) return `${Math.floor(diff / 1000)}s ago`
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`
  if (diff < 604_800_000) return `${Math.floor(diff / 86_400_000)}d ago`

  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: date.getFullYear() !== new Date().getFullYear() ? 'numeric' : undefined,
  })
}

export function formatMs(ms: number | null | undefined): string {
  if (ms == null) return '—'
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  const val = bytes / Math.pow(1024, i)
  return `${val < 10 ? val.toFixed(1) : Math.round(val)} ${units[i]}`
}

export function statusColor(status: JobStatus): string {
  const map: Record<JobStatus, string> = {
    pending: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    scheduled: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
    running: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200',
    completed: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    failed: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
    dead: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200',
  }
  return map[status] || 'bg-gray-100 text-gray-800'
}

export function planBadgeColor(plan: TenantPlan): string {
  const map: Record<TenantPlan, string> = {
    free: 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-200',
    starter: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-200',
    pro: 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-200',
    business: 'bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-200',
  }
  return map[plan] || 'bg-gray-100 text-gray-700'
}

export function truncateUrl(url: string, maxLen = 60): string {
  if (url.length <= maxLen) return url
  try {
    const u = new URL(url)
    const host = u.hostname
    const path = u.pathname
    const available = maxLen - host.length - 3
    if (available <= 0) return host.slice(0, maxLen - 1) + '…'
    if (path.length <= available) return host + path
    return host + path.slice(0, available - 1) + '…'
  } catch {
    return url.slice(0, maxLen - 1) + '…'
  }
}

export function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return String(n)
}
