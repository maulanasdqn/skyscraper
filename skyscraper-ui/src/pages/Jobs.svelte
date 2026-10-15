<script lang="ts">
  import Badge from '@components/ui/Badge.svelte'
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import Select from '@components/ui/Select.svelte'
  import { getTenantApiKey } from '@lib/stores/auth.svelte'
  import {
    getJobs, isLoading, getPage, getTotalPages, getTotal,
    getStatusFilter, setStatusFilter, setPage, fetchJobs,
  } from '@lib/stores/jobs.svelte'
  import { cancelJob, retryJob } from '@lib/api/skyscraper'
  import { formatDate, formatMs, truncateUrl } from '@lib/utils/format'
  import type { JobStatus } from '@lib/types'

  const statusOpts = [
    { value: '', label: 'All statuses' },
    { value: 'pending', label: 'Pending' },
    { value: 'running', label: 'Running' },
    { value: 'completed', label: 'Completed' },
    { value: 'failed', label: 'Failed' },
    { value: 'dead', label: 'Dead' },
  ]

  const statusBadge: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow',
    failed: 'red', dead: 'gray', scheduled: 'blue',
  }

  let filterVal = $state(getStatusFilter())

  function load() {
    const key = getTenantApiKey()
    if (key) fetchJobs(key)
  }

  function onFilterChange() {
    setStatusFilter(filterVal as JobStatus | '')
    load()
  }

  function goPage(p: number) {
    setPage(p)
    load()
  }

  async function handleCancel(id: string) {
    const key = getTenantApiKey()
    if (key) { await cancelJob(key, id); load() }
  }

  async function handleRetry(id: string) {
    const key = getTenantApiKey()
    if (key) { await retryJob(key, id); load() }
  }

  function navigate(route: string) { window.location.hash = route }

  $effect(() => { load() })
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <div class="w-48">
      <Select options={statusOpts} bind:value={filterVal} />
    </div>
    <div class="flex items-center gap-3">
      <span class="text-sm text-gray-500">{getTotal()} jobs</span>
      <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
      <Button size="sm" onclick={() => navigate('scrape')}>New Scrape</Button>
    </div>
  </div>

  {#if filterVal !== getStatusFilter()}
    <Button size="sm" variant="secondary" onclick={onFilterChange}>Apply Filter</Button>
  {/if}

  <Card>
    {#if isLoading()}
      <p class="text-sm text-gray-500 animate-pulse">Loading jobs...</p>
    {:else if getJobs().length === 0}
      <p class="text-sm text-gray-500">No jobs found</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 text-xs uppercase tracking-wider">
              <th class="pb-3 pr-4">ID</th><th class="pb-3 pr-4">URL</th>
              <th class="pb-3 pr-4">Status</th><th class="pb-3 pr-4">Priority</th>
              <th class="pb-3 pr-4">Duration</th><th class="pb-3 pr-4">Created</th>
              <th class="pb-3">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each getJobs() as job}
              <tr class="hover:bg-gray-800/40 transition-colors">
                <td class="py-2.5 pr-4">
                  <button class="text-indigo-400 hover:underline font-mono text-xs"
                    onclick={() => navigate(`jobs/${job.id}`)}>
                    {job.id.slice(0, 8)}
                  </button>
                </td>
                <td class="py-2.5 pr-4 text-gray-300 max-w-xs truncate">{truncateUrl(job.request.url, 45)}</td>
                <td class="py-2.5 pr-4"><Badge text={job.status} color={statusBadge[job.status] ?? 'gray'} /></td>
                <td class="py-2.5 pr-4 text-gray-400">{job.priority}</td>
                <td class="py-2.5 pr-4 text-gray-400">{formatMs(job.result?.elapsed_ms)}</td>
                <td class="py-2.5 pr-4 text-gray-500">{formatDate(job.created_at)}</td>
                <td class="py-2.5 space-x-1">
                  {#if job.status === 'pending'}
                    <Button size="sm" variant="danger" onclick={() => handleCancel(job.id)}>Cancel</Button>
                  {/if}
                  {#if job.status === 'failed' || job.status === 'dead'}
                    <Button size="sm" variant="secondary" onclick={() => handleRetry(job.id)}>Retry</Button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div class="flex items-center justify-between mt-4 pt-3 border-t border-gray-800">
        <span class="text-xs text-gray-500">Page {getPage()} of {getTotalPages()}</span>
        <div class="flex gap-2">
          <Button size="sm" variant="ghost" disabled={getPage() <= 1}
            onclick={() => goPage(getPage() - 1)}>Prev</Button>
          <Button size="sm" variant="ghost" disabled={getPage() >= getTotalPages()}
            onclick={() => goPage(getPage() + 1)}>Next</Button>
        </div>
      </div>
    {/if}
  </Card>
</div>
