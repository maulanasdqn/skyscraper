<script lang="ts">
  import StatCard from '@components/ui/stat-card.svelte'
  import Button from '@components/ui/button.svelte'
  import { getAdminKey } from '@lib/stores/auth.svelte'
  import { showToast } from '@lib/stores/toast.svelte'
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
    if (key) { await flushCache(key); showToast('success', 'Cache flushed'); load() }
  }

  async function handleRetryDead() {
    const key = getAdminKey()
    if (key) { const r = await retryDead(key); showToast('success', `Retried ${r.retried} jobs`); load() }
  }

  function toggleAuto() {
    autoRefresh = !autoRefresh
    if (autoRefresh) refreshTimer = setInterval(load, 5000)
    else if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null }
  }

  $effect(() => { load() })
  $effect(() => { return () => { if (refreshTimer) clearInterval(refreshTimer) } })

  let hitRate = $derived(
    metrics && (metrics.cache_hits + metrics.cache_misses) > 0
      ? Math.round((metrics.cache_hits / (metrics.cache_hits + metrics.cache_misses)) * 100)
      : 0
  )
</script>

<div class="space-y-4 animate-fade-in">
  <div class="flex items-center justify-end">
    <div class="flex gap-2">
      <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={load}>Refresh</button>
      <button
        class="text-xs px-2.5 py-1 rounded-lg transition-all
          {autoRefresh ? 'bg-accent/10 text-accent border border-accent/20' : 'text-text-muted hover:text-text-secondary border border-transparent'}"
        onclick={toggleAuto}>{autoRefresh ? 'Auto ON' : 'Auto OFF'}</button>
    </div>
  </div>

  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
    <StatCard label="Processed" value={formatNumber(metrics?.jobs_processed ?? 0)} />
    <StatCard label="Failed" value={formatNumber(metrics?.jobs_failed ?? 0)}
      trend={metrics && metrics.jobs_failed > 0 ? 'down' : 'neutral'} />
    <StatCard label="Timed Out" value={formatNumber(metrics?.jobs_timed_out ?? 0)} />
    <StatCard label="Cache Hit" value="{hitRate}%"
      subtitle="{metrics?.cache_hits ?? 0} / {(metrics?.cache_hits ?? 0) + (metrics?.cache_misses ?? 0)}" />
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
    <div class="rounded-xl border border-border-subtle bg-surface p-4 space-y-3">
      <h3 class="text-xs text-text-muted uppercase tracking-wider">Queue</h3>
      {#each [
        { label: 'Pending', val: queue?.pending, cls: 'text-warning' },
        { label: 'Processing', val: queue?.processing, cls: 'text-info' },
        { label: 'Scheduled', val: queue?.scheduled, cls: 'text-text-secondary' },
        { label: 'Dead', val: queue?.dead, cls: 'text-danger' },
      ] as item}
        <div class="flex justify-between text-sm">
          <span class="text-text-muted">{item.label}</span>
          <span class="{item.cls} tabular-nums">{item.val ?? '—'}</span>
        </div>
      {/each}
      {#if queue && (queue.dead ?? 0) > 0}
        <Button size="sm" variant="secondary" onclick={handleRetryDead}>Retry Dead</Button>
      {/if}
    </div>

    <div class="rounded-xl border border-border-subtle bg-surface p-4 space-y-3">
      <h3 class="text-xs text-text-muted uppercase tracking-wider">Cache</h3>
      <div class="flex justify-between text-sm">
        <span class="text-text-muted">Cached URLs</span>
        <span class="text-text-secondary tabular-nums">{cache?.cached_urls ?? '—'}</span>
      </div>
      <div class="flex justify-between text-sm">
        <span class="text-text-muted">TTL</span>
        <span class="text-text-secondary">{cache?.ttl_secs ?? '—'}s</span>
      </div>
      <Button size="sm" variant="danger" onclick={handleFlush}>Flush Cache</Button>
    </div>
  </div>
</div>
