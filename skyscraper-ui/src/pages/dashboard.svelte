<script lang="ts">
  import StatCard from '@components/ui/stat-card.svelte'
  import Card from '@components/ui/card.svelte'
  import Badge from '@components/ui/badge.svelte'
  import { getHealth } from '@lib/api/skyscraper'
  import { getMetrics, getQueueStats, listAllJobs } from '@lib/api/skyscraper'
  import { getCosmiumHealth } from '@lib/api/cosmium'
  import { getAdminKey, getCosmiumUrl, getCosmiumApiKey } from '@lib/stores/auth.svelte'
  import { formatDate, formatMs, truncateUrl } from '@lib/utils/format'
  import type { Job, Metrics, QueueStats } from '@lib/types'

  let metrics = $state<Metrics | null>(null)
  let queue = $state<QueueStats | null>(null)
  let recentJobs = $state<Job[]>([])
  let skyHealth = $state<string>('...')
  let cosHealth = $state<string>('...')

  const statusColor: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow',
    failed: 'red', dead: 'gray', scheduled: 'blue',
  }

  const icons = {
    processed: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
    pending: 'M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z',
    cache: 'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4',
    failed: 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z',
  }

  async function load() {
    const key = getAdminKey()
    const [m, q, jobs, h] = await Promise.allSettled([
      key ? getMetrics(key) : Promise.reject(),
      key ? getQueueStats(key) : Promise.reject(),
      key ? listAllJobs(key, { per_page: 6 }) : Promise.reject(),
      getHealth(),
    ])
    if (m.status === 'fulfilled') metrics = m.value
    if (q.status === 'fulfilled') queue = q.value
    if (jobs.status === 'fulfilled') recentJobs = jobs.value.data
    skyHealth = h.status === 'fulfilled' ? 'online' : 'offline'
    try {
      await getCosmiumHealth(getCosmiumUrl(), getCosmiumApiKey())
      cosHealth = 'online'
    } catch { cosHealth = 'offline' }
  }

  $effect(() => { load() })
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold text-text-primary">Dashboard</h1>
    <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={load}>Refresh</button>
  </div>

  <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
    <StatCard label="Jobs Processed" value={metrics?.jobs_processed ?? '—'} icon={icons.processed} />
    <StatCard label="Queue Pending" value={queue?.pending ?? '—'} icon={icons.pending} />
    <StatCard label="Cache Hits" value={metrics?.cache_hits ?? '—'} icon={icons.cache}
      subtitle={metrics ? `${metrics.cache_misses} misses` : ''} />
    <StatCard label="Failed" value={metrics?.jobs_failed ?? '—'} icon={icons.failed}
      trend={metrics && metrics.jobs_failed > 0 ? 'down' : 'neutral'} />
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
    <Card title="Services">
      <div class="space-y-3">
        {#each [
          { name: 'SkyScraper API', status: skyHealth },
          { name: 'Cosmium Browser', status: cosHealth },
        ] as svc}
          <div class="flex items-center justify-between">
            <span class="text-sm text-text-secondary">{svc.name}</span>
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full {svc.status === 'online' ? 'bg-success' : 'bg-danger'}"></span>
              <span class="text-xs text-text-muted">{svc.status}</span>
            </div>
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Queue">
      <div class="space-y-2.5 text-sm">
        {#each [
          ['Pending', queue?.pending, 'text-warning'],
          ['Processing', queue?.processing, 'text-info'],
          ['Scheduled', queue?.scheduled, 'text-text-secondary'],
          ['Dead Letter', queue?.dead, 'text-danger'],
        ] as [label, val, cls]}
          <div class="flex justify-between">
            <span class="text-text-muted">{label}</span>
            <span class="{cls} tabular-nums font-medium">{val ?? '—'}</span>
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Quick Actions">
      <div class="space-y-2.5">
        <a href="#playground" class="flex items-center gap-2 text-sm text-accent hover:text-accent-hover transition-colors">
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path d="M13 10V3L4 14h7v7l9-11h-7z" stroke-linecap="round" stroke-linejoin="round"/></svg>
          New Scrape
        </a>
        <a href="#jobs" class="block text-sm text-text-secondary hover:text-text-primary transition-colors">View Jobs</a>
        <a href="#metrics" class="block text-sm text-text-secondary hover:text-text-primary transition-colors">Metrics</a>
      </div>
    </Card>
  </div>

  <Card title="Recent Jobs">
    {#if recentJobs.length === 0}
      <p class="text-sm text-text-muted">No jobs yet. Start scraping from the Playground.</p>
    {:else}
      <div class="overflow-x-auto -mx-5 -mb-5">
        <table class="w-full">
          <thead>
            <tr class="border-b border-border-subtle">
              {#each ['URL', 'Status', 'Duration', 'Created'] as h}
                <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-5 pb-3">{h}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each recentJobs as job}
              <tr class="border-b border-border-subtle/40 hover:bg-surface-raised/40 cursor-pointer transition-colors"
                  onclick={() => window.location.hash = `jobs/${job.id}`}>
                <td class="px-5 py-3 text-sm text-text-secondary">{truncateUrl(job.request.url, 50)}</td>
                <td class="px-5 py-3"><Badge text={job.status} color={statusColor[job.status] ?? 'gray'} /></td>
                <td class="px-5 py-3 text-sm text-text-muted tabular-nums">{formatMs(job.result?.elapsed_ms)}</td>
                <td class="px-5 py-3 text-sm text-text-muted">{formatDate(job.created_at)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </Card>
</div>
