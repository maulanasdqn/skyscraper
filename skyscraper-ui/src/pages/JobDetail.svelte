<script lang="ts">
  import Badge from '@components/ui/Badge.svelte'
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import { ensureTenant } from '@lib/stores/auth.svelte'
  import { getJob, cancelJob, retryJob, getScreenshotUrl } from '@lib/api/skyscraper'
  import { formatDate, formatMs, formatBytes } from '@lib/utils/format'
  import type { Job } from '@lib/types'

  let { jobId = '' }: { jobId?: string } = $props()

  let job = $state<Job | null>(null)
  let loading = $state(true)
  let error = $state('')

  const statusBadge: Record<string, 'green' | 'yellow' | 'red' | 'blue' | 'gray'> = {
    completed: 'green', running: 'blue', pending: 'yellow',
    failed: 'red', dead: 'gray', scheduled: 'blue',
  }

  async function load() {
    if (!jobId) { error = 'No job ID'; loading = false; return }
    loading = true
    try {
      const key = await ensureTenant()
      job = await getJob(key, jobId)
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load job'
    }
    loading = false
  }

  async function handleCancel() {
    const key = await ensureTenant()
    if (key && job) { await cancelJob(key, job.id); load() }
  }

  async function handleRetry() {
    const key = await ensureTenant()
    if (key && job) { await retryJob(key, job.id); load() }
  }

  $effect(() => { if (jobId) load() })
</script>

<div class="space-y-4">
  <button class="text-sm text-indigo-400 hover:underline" onclick={() => window.location.hash = 'jobs'}>
    ← Back to Jobs
  </button>

  {#if loading}
    <p class="text-gray-500 animate-pulse">Loading...</p>
  {:else if error}
    <Card><p class="text-red-400">{error}</p></Card>
  {:else if job}
    <div class="flex items-center gap-3 mb-2">
      <h2 class="text-lg font-semibold text-gray-100 font-mono">{job.id.slice(0, 12)}…</h2>
      <Badge text={job.status} color={statusBadge[job.status] ?? 'gray'} />
      <div class="ml-auto flex gap-2">
        {#if job.status === 'pending'}
          <Button size="sm" variant="danger" onclick={handleCancel}>Cancel</Button>
        {/if}
        {#if job.status === 'failed' || job.status === 'dead'}
          <Button size="sm" variant="secondary" onclick={handleRetry}>Retry</Button>
        {/if}
        <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <Card title="Metadata">
        <dl class="space-y-2 text-sm">
          {#each [
            ['Priority', job.priority],
            ['Attempts', `${job.attempts} / ${job.max_retries}`],
            ['Created', formatDate(job.created_at)],
            ['Started', formatDate(job.started_at)],
            ['Completed', formatDate(job.completed_at)],
            ['Profile', job.profile_name ?? '—'],
            ['Carrier', job.carrier_id?.slice(0, 8) ?? '—'],
          ] as [label, val]}
            <div class="flex justify-between">
              <dt class="text-gray-500">{label}</dt>
              <dd class="text-gray-200">{val}</dd>
            </div>
          {/each}
          {#if job.error}
            <div class="pt-2 border-t border-gray-800">
              <dt class="text-red-400 text-xs mb-1">Error</dt>
              <dd class="text-red-300 text-xs font-mono bg-red-950/30 p-2 rounded">{job.error}</dd>
            </div>
          {/if}
        </dl>
      </Card>

      <Card title="Request">
        <dl class="space-y-2 text-sm">
          <div>
            <dt class="text-gray-500 text-xs">URL</dt>
            <dd class="text-indigo-400 break-all">{job.request.url}</dd>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Wait</span>
            <span class="text-gray-300">{job.request.wait_ms}ms</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Screenshot</span>
            <span class="text-gray-300">{job.request.screenshot ? 'Yes' : 'No'}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Include HTML</span>
            <span class="text-gray-300">{job.request.include_html ? 'Yes' : 'No'}</span>
          </div>
          {#if job.request.extract.length}
            <div>
              <dt class="text-gray-500 text-xs">Selectors</dt>
              <dd class="text-gray-300 font-mono text-xs">{job.request.extract.join(', ')}</dd>
            </div>
          {/if}
        </dl>
      </Card>
    </div>

    {#if job.result}
      <Card title="Result">
        <dl class="space-y-2 text-sm">
          <div class="flex gap-4 flex-wrap">
            <span class="text-gray-500">Status: <span class="text-gray-200">{job.result.http_status}</span></span>
            <span class="text-gray-500">Time: <span class="text-gray-200">{formatMs(job.result.elapsed_ms)}</span></span>
            <span class="text-gray-500">HTML: <span class="text-gray-200">{formatBytes(job.result.html_length)}</span></span>
            <span class="text-gray-500">Profile: <span class="text-gray-200">{job.result.profile_used}</span></span>
          </div>
          {#if job.result.proxy_used}
            <div>
              <dt class="text-gray-500 text-xs">Proxy</dt>
              <dd class="text-gray-300 font-mono text-xs break-all">{job.result.proxy_used}</dd>
            </div>
          {/if}
          <div>
            <dt class="text-gray-500 text-xs mb-1">Extracted Data</dt>
            <dd class="bg-gray-800 rounded-lg p-3 overflow-x-auto">
              <pre class="text-xs text-green-400 whitespace-pre-wrap">{JSON.stringify(job.result.extracted, null, 2)}</pre>
            </dd>
          </div>
          {#if job.result.screenshot_base64}
            <div>
              <dt class="text-gray-500 text-xs mb-1">Screenshot</dt>
              <img src={getScreenshotUrl(job.id)} alt="Screenshot"
                class="rounded border border-gray-700 max-w-full" />
            </div>
          {/if}
        </dl>
      </Card>
    {/if}
  {/if}
</div>
