<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import { ensureTenant } from '@lib/stores/auth.svelte'
  import { getJob, cancelJob, retryJob, getScreenshotUrl } from '@lib/api/skyscraper'
  import { formatDate, formatMs, formatBytes } from '@lib/utils/format'
  import type { Job } from '@lib/types'

  let { jobId = '' }: { jobId?: string } = $props()

  let job = $state<Job | null>(null)
  let loading = $state(true)
  let error = $state('')
  let pollTimer = $state<ReturnType<typeof setInterval> | null>(null)

  const TERMINAL = new Set(['completed', 'failed', 'dead'])
  const statusColor: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow', failed: 'red', dead: 'gray',
  }
  let isLive = $derived(!!job && !TERMINAL.has(job.status))

  async function load(silent = false) {
    if (!jobId) { error = 'No job ID'; loading = false; return }
    if (!silent) loading = true
    try {
      const key = await ensureTenant()
      job = await getJob(key, jobId)
      if (job && TERMINAL.has(job.status)) stopPoll()
    } catch (e) { error = e instanceof Error ? e.message : 'Failed to load job' }
    if (!silent) loading = false
  }

  function startPoll() { stopPoll(); pollTimer = setInterval(() => load(true), 1000) }
  function stopPoll() { if (pollTimer) { clearInterval(pollTimer); pollTimer = null } }

  async function handleCancel() {
    const key = await ensureTenant()
    if (key && job) { await cancelJob(key, job.id); load() }
  }

  async function handleRetry() {
    const key = await ensureTenant()
    if (key && job) { await retryJob(key, job.id); load() }
  }

  $effect(() => {
    if (jobId) { load().then(() => { if (isLive) startPoll() }) }
    return () => stopPoll()
  })
</script>

<div class="space-y-5">
  <div class="flex items-center gap-3">
    <a href="#jobs" class="text-xs text-text-muted hover:text-text-secondary transition-colors">Jobs</a>
    <span class="text-text-muted/50">/</span>
    <span class="text-xs text-text-secondary font-mono">{jobId.slice(0, 12)}</span>
    {#if isLive}
      <span class="flex items-center gap-1.5 text-[11px] text-success ml-2">
        <span class="relative flex h-1.5 w-1.5">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75"></span>
          <span class="relative inline-flex rounded-full h-1.5 w-1.5 bg-success"></span>
        </span>
        live
      </span>
    {/if}
  </div>

  {#if loading}
    <div class="p-12 text-center"><svg class="animate-spin h-5 w-5 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg></div>
  {:else if error}
    <div class="rounded-xl border border-danger/20 bg-danger/5 p-5 text-sm text-danger">{error}</div>
  {:else if job}
    <div class="flex items-center gap-2">
      <Badge text={job.status} color={statusColor[job.status] ?? 'gray'} />
      {#if job.status === 'pending'}
        <Button size="sm" variant="danger" onclick={handleCancel}>Cancel</Button>
      {/if}
      {#if job.status === 'failed' || job.status === 'dead'}
        <Button size="sm" variant="secondary" onclick={handleRetry}>Retry</Button>
      {/if}
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <div class="rounded-xl border border-border-subtle bg-surface p-5 space-y-2.5">
        <h3 class="text-xs text-text-muted uppercase tracking-wider mb-3">Request</h3>
        <div class="text-sm text-accent break-all">{job.request.url}</div>
        {#each [
          ['Wait', `${job.request.wait_ms}ms`],
          ['Priority', job.priority],
          ['Attempts', `${job.attempts} / ${job.max_retries}`],
        ] as [k, v]}
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">{k}</span><span class="text-text-secondary">{v}</span>
          </div>
        {/each}
        {#if job.request.extract.length}
          <div class="text-xs text-text-muted">Selectors: <span class="text-text-secondary font-mono">{job.request.extract.join(', ')}</span></div>
        {/if}
      </div>

      <div class="rounded-xl border border-border-subtle bg-surface p-5 space-y-2.5">
        <h3 class="text-xs text-text-muted uppercase tracking-wider mb-3">Metadata</h3>
        {#each [
          ['Created', formatDate(job.created_at)],
          ['Started', formatDate(job.started_at)],
          ['Completed', formatDate(job.completed_at)],
          ['Profile', job.profile_name ?? '—'],
        ] as [k, v]}
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">{k}</span><span class="text-text-secondary">{v}</span>
          </div>
        {/each}
        {#if job.error}
          <div class="mt-2 p-3 rounded-lg bg-danger/5 border border-danger/10 text-xs text-danger font-mono">{job.error}</div>
        {/if}
      </div>
    </div>

    {#if job.result}
      <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
        <div class="flex items-center gap-5 px-5 py-3.5 border-b border-border-subtle text-sm">
          <span class="text-text-muted">HTTP <span class="text-text-secondary font-medium">{job.result.http_status}</span></span>
          <span class="text-text-muted">{formatMs(job.result.elapsed_ms)}</span>
          <span class="text-text-muted">{formatBytes(job.result.html_length)}</span>
          <span class="text-text-muted">{job.result.profile_used}</span>
        </div>
        <div class="relative">
          <div class="absolute top-3 right-4 text-[10px] text-text-muted uppercase tracking-wider">JSON</div>
          <pre class="p-5 text-xs text-text-secondary font-mono overflow-x-auto max-h-96 leading-relaxed">{JSON.stringify(job.result.extracted, null, 2)}</pre>
        </div>
        {#if job.result.screenshot_base64}
          <div class="p-5 border-t border-border-subtle">
            <img src={getScreenshotUrl(job.id)} alt="Screenshot" class="rounded-xl border border-border-subtle max-w-full" />
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>
