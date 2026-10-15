<script lang="ts">
  import Button from '@components/ui/button.svelte'
  import ScrapeResult from '@components/ui/scrape-result.svelte'
  import { ensureTenant, getCosmiumApiKey } from '@lib/stores/auth.svelte'
  import { submitJob, scrapeSync, type SubmitJobData } from '@lib/api/skyscraper'
  import { aiFormat } from '@lib/api/cosmium'
  import { extractStructuredData, extractPageText } from '@lib/utils/html-cleaner'
  import type { Job, ScrapeResult as ScrapeResultType } from '@lib/types'

  let url = $state('')
  let aiPrompt = $state('')
  let extractors = $state('')
  let waitMs = $state('1500')
  let screenshot = $state(false)
  let mode = $state<'sync' | 'async'>('sync')
  let extractMode = $state<'ai' | 'css'>('ai')
  let showAdvanced = $state(false)

  let phase = $state<'idle' | 'scraping' | 'extracting' | 'done' | 'error'>('idle')
  let scrape = $state<ScrapeResultType | null>(null)
  let jobId = $state('')
  let error = $state('')
  let aiResult = $state<{ data: Record<string, unknown>; model: string } | null>(null)
  let wallTimeMs = $state(0)

  function buildRequest(targetUrl: string): SubmitJobData {
    return {
      url: targetUrl,
      extract: extractMode === 'css' && extractors
        ? extractors.split(',').map((s) => s.trim()).filter(Boolean) : [],
      wait_ms: parseInt(waitMs) || 1500,
      screenshot,
      include_html: true,
    }
  }

  async function handleSubmit() {
    if (!url) return
    phase = 'scraping'; scrape = null; error = ''; aiResult = null; jobId = ''; wallTimeMs = 0
    let key: string
    try { key = await ensureTenant() } catch { error = 'Failed to provision'; phase = 'error'; return }
    const t0 = performance.now()
    try {
      if (mode === 'async') {
        const job = await submitJob(key, buildRequest(url))
        window.location.hash = `jobs/${job.id}`
        return
      }
      const res = await scrapeSync(key, buildRequest(url), 90000)
      wallTimeMs = Math.round(performance.now() - t0)
      scrape = res.result ?? null
      jobId = res.job_id
      if (extractMode === 'ai' && scrape?.html) {
        const structured = extractStructuredData(scrape.html)
        if (structured && !aiPrompt) {
          aiResult = { data: structured, model: 'json-ld (instant)' }
        } else {
          phase = 'extracting'
          const prompt = aiPrompt || 'Extract: product name, price, currency, rating, review count, availability, description, images. Omit fields without values.'
          try {
            aiResult = await aiFormat('', url, prompt, { content: extractPageText(scrape.html) }, getCosmiumApiKey())
          } catch (e) {
            aiResult = { data: { error: e instanceof Error ? e.message : 'AI failed' }, model: '' }
          }
        }
      }
      phase = 'done'
    } catch (e) {
      error = e instanceof Error ? e.message : 'Scrape failed'
      phase = 'error'
    }
  }

  function onKey(e: KeyboardEvent) { if (e.key === 'Enter' && phase !== 'scraping') handleSubmit() }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-xl font-bold text-text-primary">Playground</h1>
    <p class="text-sm text-text-muted mt-1">Scrape any URL with anti-detection browser</p>
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface">
    <div class="p-4 space-y-3">
      <div class="flex gap-2">
        <div class="relative flex-1">
          <input type="text" bind:value={url} onkeydown={onKey}
            placeholder="Enter URL to scrape..."
            class="w-full rounded-xl border border-border-subtle bg-surface-raised pl-4 pr-4 py-3 text-sm
              text-text-primary placeholder-text-muted
              focus:outline-none focus:ring-1 focus:ring-accent/40 focus:border-accent/40" />
        </div>
        <Button variant="primary" size="lg" onclick={handleSubmit}
          loading={phase === 'scraping' || phase === 'extracting'}
          disabled={phase === 'scraping' || phase === 'extracting' || !url}>
          {phase === 'scraping' ? 'Scraping...' : phase === 'extracting' ? 'Extracting...' : 'Scrape'}
        </Button>
      </div>

      {#if extractMode === 'ai'}
        <div class="relative">
          <span class="absolute left-3 top-2.5 text-[10px] text-text-muted uppercase tracking-wider">AI Prompt</span>
          <textarea bind:value={aiPrompt} rows={1}
            placeholder="e.g. Extract product name, price, rating  (leave blank for auto)"
            class="w-full rounded-xl border border-border-subtle bg-surface-raised pl-3 pr-3 pt-7 pb-2 text-sm
              text-text-primary placeholder-text-muted resize-none
              focus:outline-none focus:ring-1 focus:ring-accent/40 focus:border-accent/40"></textarea>
        </div>
      {:else}
        <div class="relative">
          <span class="absolute left-3 top-2.5 text-[10px] text-text-muted uppercase tracking-wider">CSS Selectors</span>
          <input bind:value={extractors} placeholder="e.g. h1, .price, #description"
            class="w-full rounded-xl border border-border-subtle bg-surface-raised pl-3 pr-3 pt-7 pb-2 text-sm
              text-text-primary placeholder-text-muted
              focus:outline-none focus:ring-1 focus:ring-accent/40 focus:border-accent/40" />
        </div>
      {/if}
    </div>

    <div class="flex items-center justify-between px-4 py-2.5 border-t border-border-subtle bg-surface-raised/30">
      <div class="flex items-center gap-3">
        <div class="flex gap-0.5 bg-surface-overlay/50 rounded-lg p-0.5">
          {#each [{ id: 'ai', label: 'AI' }, { id: 'css', label: 'CSS' }] as m}
            <button class="px-2.5 py-1 text-[11px] rounded-md transition-all
              {extractMode === m.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
              onclick={() => (extractMode = m.id as 'ai' | 'css')}>{m.label}</button>
          {/each}
        </div>
        <div class="flex gap-0.5 bg-surface-overlay/50 rounded-lg p-0.5">
          {#each [{ id: 'sync', label: 'Sync' }, { id: 'async', label: 'Async' }] as m}
            <button class="px-2.5 py-1 text-[11px] rounded-md transition-all
              {mode === m.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
              onclick={() => (mode = m.id as 'sync' | 'async')}>{m.label}</button>
          {/each}
        </div>
      </div>
      <button class="text-[11px] text-text-muted hover:text-text-secondary transition-colors flex items-center gap-1"
        onclick={() => (showAdvanced = !showAdvanced)}>
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <path d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        Advanced
      </button>
    </div>

    {#if showAdvanced}
      <div class="px-4 py-3 border-t border-border-subtle flex items-center gap-6">
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-text-muted uppercase tracking-wider">Wait (ms)</label>
          <input type="number" bind:value={waitMs}
            class="w-20 rounded-lg border border-border-subtle bg-surface-raised px-2.5 py-1.5 text-xs text-text-primary
              focus:outline-none focus:ring-1 focus:ring-accent/40" />
        </div>
        <label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer mt-4">
          <input type="checkbox" checked={screenshot} onchange={() => screenshot = !screenshot}
            class="rounded border-border-default bg-surface-raised accent-accent" />
          Screenshot
        </label>
      </div>
    {/if}
  </div>

  {#if phase === 'scraping' || phase === 'extracting'}
    <div class="rounded-xl border border-border-subtle bg-surface p-6">
      <div class="flex items-center gap-4">
        <svg class="animate-spin h-4 w-4 text-accent shrink-0" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
        <div>
          <p class="text-sm text-text-primary">{phase === 'scraping' ? 'Scraping page...' : 'AI extracting data...'}</p>
          <p class="text-xs text-text-muted mt-0.5">
            {phase === 'scraping' ? 'Loading page with anti-detection browser' : 'Processing with Netra AI'}
          </p>
        </div>
      </div>
    </div>
  {/if}

  {#if phase === 'error'}
    <div class="rounded-xl border border-danger/20 bg-danger/5 p-4 text-sm text-danger">{error}</div>
  {/if}

  {#if scrape && (phase === 'done' || phase === 'extracting')}
    <ScrapeResult {scrape} {jobId} {aiResult} aiLoading={phase === 'extracting'} {wallTimeMs} />
  {/if}
</div>
