<script lang="ts">
  import StatCard from '@components/ui/StatCard.svelte'
  import Card from '@components/ui/Card.svelte'
  import Badge from '@components/ui/Badge.svelte'
  import Button from '@components/ui/Button.svelte'
  import { getHealth } from '@lib/api/skyscraper'
  import { getMetrics, getQueueStats, listAllJobs, flushCache } from '@lib/api/skyscraper'
  import { getCosmiumHealth } from '@lib/api/cosmium'
  import { getAdminKey, getCosmiumUrl, getCosmiumApiKey } from '@lib/stores/auth.svelte'
  import { formatDate, formatMs, truncateUrl } from '@lib/utils/format'
  import type { Job, Metrics, QueueStats } from '@lib/types'

  let metrics = $state<Metrics | null>(null)
  let queue = $state<QueueStats | null>(null)
  let recentJobs = $state<Job[]>([])
  let skyHealth = $state<string>('...')
  let cosHealth = $state<string>('...')
  let loading = $state(true)

  const statusBadge: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow',
    failed: 'red', dead: 'gray', scheduled: 'blue',
  }

  async function load() {
    loading = true
    const key = getAdminKey()
    try {
      const [m, q, jobs, h] = await Promise.allSettled([
        key ? getMetrics(key) : Promise.reject('no key'),
        key ? getQueueStats(key) : Promise.reject('no key'),
        key ? listAllJobs(key, { per_page: 8 }) : Promise.reject('no key'),
        getHealth(),
      ])
      if (m.status === 'fulfilled') metrics = m.value
      if (q.status === 'fulfilled') queue = q.value
      if (jobs.status === 'fulfilled') recentJobs = jobs.value.data
      skyHealth = h.status === 'fulfilled' ? 'online' : 'offline'
    } catch { skyHealth = 'error' }

    try {
      await getCosmiumHealth(getCosmiumUrl(), getCosmiumApiKey())
      cosHealth = 'online'
    } catch { cosHealth = 'offline' }
    loading = false
  }

  async function handleFlush() {
    const key = getAdminKey()
    if (key) { await flushCache(key); load() }
  }

  $effect(() => { load() })

  function navigate(route: string) { window.location.hash = route }
</script>

<div class="space-y-6">
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
    <StatCard label="Jobs Processed" value={metrics?.jobs_processed ?? '—'} />
    <StatCard label="Queue Pending" value={queue?.pending ?? '—'} />
    <StatCard label="Cache Hits" value={metrics?.cache_hits ?? '—'}
      subtitle={metrics ? `${metrics.cache_misses} misses` : ''} />
    <StatCard label="Failed" value={metrics?.jobs_failed ?? '—'}
      trend={metrics && metrics.jobs_failed > 0 ? 'down' : 'neutral'} />
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
    <Card title="Service Health">
      <div class="space-y-3">
        {#each [
          { name: 'SkyScraper API', status: skyHealth },
          { name: 'Cosmium Browser', status: cosHealth },
        ] as svc}
          <div class="flex items-center justify-between">
            <span class="text-sm text-gray-300">{svc.name}</span>
            <Badge text={svc.status}
              color={svc.status === 'online' ? 'green' : svc.status === 'offline' ? 'red' : 'gray'} />
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Queue">
      <div class="space-y-2 text-sm">
        {#each [
          ['Pending', queue?.pending],
          ['Processing', queue?.processing],
          ['Scheduled', queue?.scheduled],
          ['Dead Letter', queue?.dead],
        ] as [label, val]}
          <div class="flex justify-between">
            <span class="text-gray-400">{label}</span>
            <span class="text-gray-200 font-medium">{val ?? '—'}</span>
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Quick Actions">
      <div class="space-y-3">
        <Button variant="primary" size="sm" onclick={() => navigate('scrape')}>New Scrape</Button>
        <Button variant="secondary" size="sm" onclick={handleFlush}>Flush Cache</Button>
        <Button variant="ghost" size="sm" onclick={() => load()}>Refresh</Button>
      </div>
    </Card>
  </div>

  <Card title="Recent Jobs">
    {#if recentJobs.length === 0}
      <p class="text-sm text-gray-500">No jobs yet</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 text-xs uppercase">
              <th class="pb-2">URL</th><th class="pb-2">Status</th>
              <th class="pb-2">Duration</th><th class="pb-2">Created</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each recentJobs as job}
              <tr class="hover:bg-gray-800/50 cursor-pointer"
                  onclick={() => navigate(`jobs/${job.id}`)}>
                <td class="py-2 text-gray-300">{truncateUrl(job.request.url, 50)}</td>
                <td class="py-2"><Badge text={job.status} color={statusBadge[job.status] ?? 'gray'} /></td>
                <td class="py-2 text-gray-400">{formatMs(job.result?.elapsed_ms)}</td>
                <td class="py-2 text-gray-500">{formatDate(job.created_at)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </Card>
</div>
