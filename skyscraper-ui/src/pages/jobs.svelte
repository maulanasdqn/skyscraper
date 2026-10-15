<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import { ensureTenant } from '@lib/stores/auth.svelte'
  import {
    getJobs, isLoading, getPage, getTotalPages, getTotal,
    getStatusFilter, setStatusFilter, setPage, fetchJobs,
  } from '@lib/stores/jobs.svelte'
  import { cancelJob, retryJob } from '@lib/api/skyscraper'
  import { formatDate, formatMs, truncateUrl } from '@lib/utils/format'
  import type { JobStatus } from '@lib/types'

  const statusColor: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow',
    failed: 'red', dead: 'gray', scheduled: 'blue',
  }

  let filterVal = $state(getStatusFilter())

  async function load() {
    const key = await ensureTenant()
    if (key) fetchJobs(key)
  }

  function onFilterChange(val: string) {
    filterVal = val
    setStatusFilter(val as JobStatus | '')
    load()
  }

  function goPage(p: number) { setPage(p); load() }

  async function handleCancel(id: string) {
    const key = await ensureTenant()
    if (key) { await cancelJob(key, id); load() }
  }

  async function handleRetry(id: string) {
    const key = await ensureTenant()
    if (key) { await retryJob(key, id); load() }
  }

  $effect(() => { load() })
</script>

<div class="space-y-5">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold text-text-primary">Jobs</h1>
    <div class="flex items-center gap-3">
      <select
        value={filterVal}
        onchange={(e) => onFilterChange((e.target as HTMLSelectElement).value)}
        class="rounded-xl border border-border-subtle bg-surface-raised px-3 py-1.5 text-xs
          text-text-secondary focus:outline-none focus:ring-1 focus:ring-accent/40"
      >
        <option value="">All</option>
        <option value="pending">Pending</option>
        <option value="running">Running</option>
        <option value="completed">Completed</option>
        <option value="failed">Failed</option>
      </select>
      <span class="text-xs text-text-muted tabular-nums">{getTotal()} total</span>
    </div>
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
    {#if isLoading()}
      <div class="p-12 text-center">
        <svg class="animate-spin h-5 w-5 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
      </div>
    {:else if getJobs().length === 0}
      <div class="p-12 text-center text-sm text-text-muted">No jobs found</div>
    {:else}
      <table class="w-full">
        <thead>
          <tr class="border-b border-border-subtle">
            {#each ['URL', 'Status', 'Time', 'Created', ''] as h}
              <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-5 py-3">{h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each getJobs() as job}
            <tr class="border-b border-border-subtle/40 hover:bg-surface-raised/40 transition-colors">
              <td class="px-5 py-3">
                <a href="#{`jobs/${job.id}`}" class="text-sm text-text-secondary hover:text-accent transition-colors">
                  {truncateUrl(job.request.url, 45)}
                </a>
              </td>
              <td class="px-5 py-3"><Badge text={job.status} color={statusColor[job.status] ?? 'gray'} /></td>
              <td class="px-5 py-3 text-sm text-text-muted tabular-nums">{formatMs(job.result?.elapsed_ms)}</td>
              <td class="px-5 py-3 text-sm text-text-muted">{formatDate(job.created_at)}</td>
              <td class="px-5 py-3">
                {#if job.status === 'pending'}
                  <button class="text-xs text-danger hover:underline" onclick={() => handleCancel(job.id)}>Cancel</button>
                {/if}
                {#if job.status === 'failed' || job.status === 'dead'}
                  <button class="text-xs text-accent hover:underline" onclick={() => handleRetry(job.id)}>Retry</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if getTotalPages() > 1}
        <div class="flex items-center justify-between px-5 py-3 border-t border-border-subtle">
          <span class="text-xs text-text-muted">{getPage()} / {getTotalPages()}</span>
          <div class="flex gap-1">
            <Button size="sm" variant="ghost" disabled={getPage() <= 1}
              onclick={() => goPage(getPage() - 1)}>Prev</Button>
            <Button size="sm" variant="ghost" disabled={getPage() >= getTotalPages()}
              onclick={() => goPage(getPage() + 1)}>Next</Button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
