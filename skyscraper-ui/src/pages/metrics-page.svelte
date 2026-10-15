<script lang="ts">
  import StatCard from '@components/ui/StatCard.svelte'
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import { getAdminKey } from '@lib/stores/auth.svelte'
  import { getMetrics, getQueueStats, getCacheStats, flushCache, retryDead } from '@lib/api/skyscraper'
  import type { Metrics, QueueStats, CacheStats } from '@lib/types'
  import { formatNumber } from '@lib/utils/format'

  let metrics = $state<Metrics | null>(null)
  let queue = $state<QueueStats | null>(null)
  let cache = $state<CacheStats | null>(null)
  let loading = $state(true)
  let autoRefresh = $state(false)
  let refreshTimer = $state<ReturnType<typeof setInterval> | null>(null)

  async function load() {
    const key = getAdminKey()
    if (!key) { loading = false; return }
    loading = true
    const [m, q, c] = await Promise.allSettled([
      getMetrics(key), getQueueStats(key), getCacheStats(key),
    ])
    if (m.status === 'fulfilled') metrics = m.value
    if (q.status === 'fulfilled') queue = q.value
    if (c.status === 'fulfilled') cache = c.value
    loading = false
  }

  async function handleFlush() {
    const key = getAdminKey()
    if (key) { await flushCache(key); load() }
  }

  async function handleRetryDead() {
    const key = getAdminKey()
    if (key) {
      const r = await retryDead(key)
      alert(`Retried ${r.retried} jobs`)
      load()
    }
  }

  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh
    if (autoRefresh) {
      refreshTimer = setInterval(load, 5000)
    } else if (refreshTimer) {
      clearInterval(refreshTimer)
      refreshTimer = null
    }
  }

  $effect(() => { load() })
  $effect(() => {
    return () => { if (refreshTimer) clearInterval(refreshTimer) }
  })

  let cacheHitRate = $derived(
    metrics && (metrics.cache_hits + metrics.cache_misses) > 0
      ? Math.round((metrics.cache_hits / (metrics.cache_hits + metrics.cache_misses)) * 100)
      : 0
  )
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div class="flex gap-2">
      <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
      <Button size="sm" variant={autoRefresh ? 'primary' : 'secondary'} onclick={toggleAutoRefresh}>
        {autoRefresh ? 'Auto-refresh ON' : 'Auto-refresh OFF'}
      </Button>
    </div>
  </div>

  {#if loading && !metrics}
    <p class="text-sm text-gray-500 animate-pulse">Loading metrics...</p>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard label="Jobs Processed" value={formatNumber(metrics?.jobs_processed ?? 0)} />
      <StatCard label="Jobs Failed" value={formatNumber(metrics?.jobs_failed ?? 0)}
        trend={metrics && metrics.jobs_failed > 0 ? 'down' : 'neutral'} />
      <StatCard label="Jobs Timed Out" value={formatNumber(metrics?.jobs_timed_out ?? 0)} />
      <StatCard label="Cache Hit Rate" value="{cacheHitRate}%"
        subtitle="{metrics?.cache_hits ?? 0} hits / {metrics?.cache_misses ?? 0} misses" />
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard label="Webhooks Sent" value={formatNumber(metrics?.webhook_deliveries ?? 0)} />
      <StatCard label="Webhook Failures" value={formatNumber(metrics?.webhook_failures ?? 0)} />
      <StatCard label="Dedup Coalesced" value={formatNumber(metrics?.dedup_coalesced ?? 0)} />
      <StatCard label="Domain Throttled" value={formatNumber(metrics?.domain_throttled ?? 0)} />
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <Card title="Queue">
        <div class="space-y-3">
          {#each [
            { label: 'Pending', val: queue?.pending, color: 'text-yellow-400' },
            { label: 'Processing', val: queue?.processing, color: 'text-blue-400' },
            { label: 'Scheduled', val: queue?.scheduled, color: 'text-indigo-400' },
            { label: 'Dead Letter', val: queue?.dead, color: 'text-red-400' },
          ] as item}
            <div class="flex justify-between items-center">
              <span class="text-sm text-gray-400">{item.label}</span>
              <span class="text-sm font-medium {item.color}">{item.val ?? '—'}</span>
            </div>
          {/each}
          {#if queue && (queue.dead ?? 0) > 0}
            <Button size="sm" variant="secondary" onclick={handleRetryDead}>Retry Dead Jobs</Button>
          {/if}
        </div>
      </Card>

      <Card title="Cache">
        <div class="space-y-3">
          <div class="flex justify-between">
            <span class="text-sm text-gray-400">Cached URLs</span>
            <span class="text-sm text-gray-200">{cache?.cached_urls ?? '—'}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-gray-400">TTL</span>
            <span class="text-sm text-gray-200">{cache?.ttl_secs ?? '—'}s</span>
          </div>
          <Button size="sm" variant="danger" onclick={handleFlush}>Flush Cache</Button>
        </div>
      </Card>
    </div>
  {/if}
</div>
