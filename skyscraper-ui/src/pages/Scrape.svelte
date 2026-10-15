<script lang="ts">
  import Button from '@components/ui/Button.svelte'
  import Input from '@components/ui/Input.svelte'
  import Card from '@components/ui/Card.svelte'
  import Select from '@components/ui/Select.svelte'
  import { ensureTenant } from '@lib/stores/auth.svelte'
  import { submitJob, scrapeSync, submitBatch, type SubmitJobData } from '@lib/api/skyscraper'
  import { formatMs } from '@lib/utils/format'
  import type { Job, ScrapeResult } from '@lib/types'

  let url = $state('')
  let extractors = $state('')
  let waitMs = $state('3000')
  let priority = $state('5')
  let screenshot = $state(false)
  let includeHtml = $state(false)
  let headful = $state(false)
  let geo = $state('')
  let mode = $state<'async' | 'sync'>('sync')
  let batchMode = $state(false)
  let batchUrls = $state('')

  let submitting = $state(false)
  let result = $state<{ job?: Job; scrape?: ScrapeResult; jobIds?: string[]; error?: string } | null>(null)

  function buildRequest(targetUrl: string): SubmitJobData {
    return {
      url: targetUrl,
      extract: extractors ? extractors.split(',').map((s) => s.trim()).filter(Boolean) : [],
      wait_ms: parseInt(waitMs) || 3000,
      priority: parseInt(priority) || 5,
      screenshot,
      include_html: includeHtml,
      headful,
      geo: geo || undefined,
    }
  }

  async function handleSubmit() {
    submitting = true
    result = null
    let key: string
    try { key = await ensureTenant() } catch { result = { error: 'Failed to provision tenant' }; submitting = false; return }
    try {
      if (batchMode) {
        const urls = batchUrls.split('\n').map((s) => s.trim()).filter(Boolean)
        if (!urls.length) { result = { error: 'Enter at least one URL' }; return }
        const jobs = urls.map((u) => buildRequest(u))
        const res = await submitBatch(key, jobs)
        result = { jobIds: res.job_ids }
      } else if (mode === 'sync') {
        if (!url) { result = { error: 'Enter a URL' }; return }
        const res = await scrapeSync(key, buildRequest(url), 60000)
        result = { scrape: res.result ?? undefined, job: { id: res.job_id, status: res.status } as Job }
      } else {
        if (!url) { result = { error: 'Enter a URL' }; return }
        const job = await submitJob(key, buildRequest(url))
        result = { job }
        window.location.hash = `jobs/${job.id}`
        return
      }
    } catch (e) {
      result = { error: e instanceof Error ? e.message : 'Scrape failed' }
    }
    submitting = false
  }
</script>

<div class="max-w-3xl space-y-4">
  <Card title="Scrape Configuration">
    <div class="space-y-4">
      <div class="flex gap-3 items-end">
        <label class="flex items-center gap-2 text-sm text-gray-300 cursor-pointer">
          <input type="checkbox" bind:checked={batchMode} class="rounded bg-gray-800 border-gray-600" />
          Batch Mode
        </label>
        {#if !batchMode}
          <div class="flex gap-2 ml-auto">
            {#each ['async', 'sync'] as m}
              <button
                class="px-3 py-1 text-xs rounded-md transition-colors
                  {mode === m ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-400 hover:bg-gray-700'}"
                onclick={() => (mode = m as 'async' | 'sync')}>{m.toUpperCase()}</button>
            {/each}
          </div>
        {/if}
      </div>

      {#if batchMode}
        <div class="flex flex-col gap-1.5">
          <label class="text-sm font-medium text-gray-300">URLs (one per line)</label>
          <textarea bind:value={batchUrls} rows={6} placeholder="https://example.com&#10;https://example.org"
            class="w-full rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100
              placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-indigo-500" />
        </div>
      {:else}
        <Input label="URL" bind:value={url} placeholder="https://example.com/page" />
      {/if}

      <Input label="CSS Selectors" bind:value={extractors} placeholder=".title, .price, h1" />

      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Input label="Wait (ms)" type="number" bind:value={waitMs} />
        <Input label="Priority" type="number" bind:value={priority} />
        <Input label="Geo" bind:value={geo} placeholder="US, ID, SG" />
        <Select label="Mode" options={[{value:'async',label:'Async'},{value:'sync',label:'Sync'}]} bind:value={mode} />
      </div>

      <div class="flex gap-4">
        {#each [
          { label: 'Screenshot', val: screenshot, set: (v: boolean) => screenshot = v },
          { label: 'Include HTML', val: includeHtml, set: (v: boolean) => includeHtml = v },
          { label: 'Headful', val: headful, set: (v: boolean) => headful = v },
        ] as opt}
          <label class="flex items-center gap-2 text-sm text-gray-400 cursor-pointer">
            <input type="checkbox" checked={opt.val} onchange={() => opt.set(!opt.val)}
              class="rounded bg-gray-800 border-gray-600" />
            {opt.label}
          </label>
        {/each}
      </div>

      <Button onclick={handleSubmit} loading={submitting} disabled={submitting}>
        {batchMode ? 'Submit Batch' : mode === 'sync' ? 'Scrape Now' : 'Submit Job'}
      </Button>
    </div>
  </Card>

  {#if result}
    <Card title="Result">
      {#if result.error}
        <p class="text-red-400 text-sm">{result.error}</p>
      {:else if result.jobIds}
        <p class="text-sm text-green-400">Batch submitted: {result.jobIds.length} jobs</p>
        <div class="mt-2 space-y-1">
          {#each result.jobIds as jid}
            <a href="#{`jobs/${jid}`}" class="block text-xs text-indigo-400 hover:underline font-mono">{jid}</a>
          {/each}
        </div>
      {:else if result.job}
        <p class="text-sm text-gray-300">Job <a href="#{`jobs/${result.job.id}`}"
          class="text-indigo-400 hover:underline font-mono">{result.job.id.slice(0, 12)}</a>
          — {result.job.status}</p>
        {#if result.scrape}
          <div class="mt-3 text-sm text-gray-400">
            <span>Time: <span class="text-gray-200">{formatMs(result.scrape.elapsed_ms)}</span></span>
            <span class="ml-4">Status: <span class="text-gray-200">{result.scrape.http_status}</span></span>
          </div>
          <pre class="mt-2 bg-gray-800 rounded-lg p-3 text-xs text-green-400 overflow-x-auto whitespace-pre-wrap"
            >{JSON.stringify(result.scrape.extracted, null, 2)}</pre>
        {/if}
      {/if}
    </Card>
  {/if}
</div>
